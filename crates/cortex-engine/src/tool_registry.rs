use std::cmp::Reverse;
use std::collections::BTreeSet;

use cortex_aql::AgentView;
use cortex_core::memtable::CellVersion;
use cortex_core::{
    CellDescriptor, CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType,
};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::options::PayloadResidency;
use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;

mod index;

pub(crate) use index::ToolIndex;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolPermission {
    Read,
    Execute,
    Write,
    ApprovalRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub scope: String,
    pub name: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub permissions: BTreeSet<ToolPermission>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredTool {
    pub cell_id: CellId,
    pub commit_seq: CommitSeq,
    pub descriptor: ToolDescriptor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRecommendation {
    pub tool: RegisteredTool,
    pub matched_terms: Vec<String>,
    pub score: u32,
    pub why_selected: String,
}

impl ToolDescriptor {
    pub fn new(
        scope: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        permissions: BTreeSet<ToolPermission>,
    ) -> Self {
        Self {
            scope: scope.into(),
            name: name.into(),
            description: description.into(),
            input_schema: None,
            output_schema: None,
            permissions,
            source: None,
        }
    }

    pub fn validate(&self) -> EngineResult<()> {
        if self.scope.trim().is_empty() {
            return Err(invalid_tool("tool scope must not be empty"));
        }
        if self.name.trim().is_empty() {
            return Err(invalid_tool("tool name must not be empty"));
        }
        if self.description.trim().is_empty() {
            return Err(invalid_tool("tool description must not be empty"));
        }
        if self.permissions.is_empty() {
            return Err(invalid_tool("tool permissions must not be empty"));
        }
        Ok(())
    }

    pub fn to_knowledge_cell(&self) -> EngineResult<KnowledgeCell> {
        self.validate()?;
        let mut lines = vec![
            format!("name={}", line_value(&self.name)),
            format!("description={}", line_value(&self.description)),
            format!("permissions={}", encode_permissions(&self.permissions)),
        ];
        if let Some(line) = optional_line("input_schema", self.input_schema.as_deref()) {
            lines.push(line);
        }
        if let Some(line) = optional_line("output_schema", self.output_schema.as_deref()) {
            lines.push(line);
        }
        let body = lines.join("\n");
        Ok(KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: self.scope.clone(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Tool,
                source: self.source.clone(),
                ..KnowledgeCellMetadata::default()
            },
            body,
        ))
    }

    pub fn from_payload(payload: &[u8]) -> EngineResult<Self> {
        let metadata = CellMetadata::from_payload(payload);
        Self::from_metadata(metadata)
    }

    pub fn from_payload_with_descriptor(
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) -> EngineResult<Self> {
        let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);
        Self::from_metadata(metadata)
    }

    pub fn from_version(version: &CellVersion) -> EngineResult<Self> {
        let metadata = CellMetadata::from_version(version);
        Self::from_metadata(metadata)
    }

    fn from_metadata(metadata: CellMetadata) -> EngineResult<Self> {
        if metadata.cell_type != KnowledgeCellType::Tool.as_str() {
            return Err(invalid_tool("cell is not a tool"));
        }
        let fields = parse_tool_fields(&metadata.body_text);
        let name = fields.get("name").cloned().unwrap_or_default();
        let description = fields
            .get("description")
            .cloned()
            .unwrap_or(metadata.body_text);
        let permissions = fields
            .get("permissions")
            .map(|value| parse_permissions(value))
            .transpose()?
            .unwrap_or_default();
        let descriptor = Self {
            scope: metadata.scope,
            name,
            description,
            input_schema: fields.get("input_schema").cloned(),
            output_schema: fields.get("output_schema").cloned(),
            permissions,
            source: metadata.source,
        };
        descriptor.validate()?;
        Ok(descriptor)
    }
}

impl Database {
    pub fn register_tool(
        &mut self,
        cell_id: CellId,
        descriptor: ToolDescriptor,
    ) -> EngineResult<RegisteredTool> {
        let cell = descriptor.to_knowledge_cell()?;
        let commit_seq = self.put_knowledge_cell(cell_id, cell)?;
        Ok(RegisteredTool {
            cell_id,
            commit_seq,
            descriptor,
        })
    }

    pub fn list_tools(&self, view: &AgentView) -> Vec<RegisteredTool> {
        let mut tools = if self.payload_residency == PayloadResidency::Lazy {
            self.memtable
                .visible_iter(self.read_txn())
                .filter_map(|version| {
                    let payload = self.payload_for_version(version).ok()?;
                    let tool = ToolIndex::record_from_payload(
                        version.cell_id,
                        version.created_seq,
                        &payload,
                        &version.descriptor,
                    )?;
                    PolicyRewrite::allows_scope(view, scope_id(&tool.descriptor.scope))
                        .then_some(tool)
                })
                .collect()
        } else {
            self.tool_index.list_tools(view)
        };
        tools.sort_by_key(|tool| tool.cell_id);
        tools
    }

    pub fn recommend_tools_for_task(
        &self,
        view: &AgentView,
        task: &str,
        limit: usize,
    ) -> Vec<ToolRecommendation> {
        if limit == 0 {
            return Vec::new();
        }
        let task_terms = tokenize(task).into_iter().collect::<BTreeSet<_>>();
        let mut recommendations = self
            .list_tools(view)
            .into_iter()
            .filter_map(|tool| {
                let descriptor_text = format!(
                    "{} {} {} {}",
                    tool.descriptor.name,
                    tool.descriptor.description,
                    tool.descriptor.input_schema.as_deref().unwrap_or(""),
                    tool.descriptor.output_schema.as_deref().unwrap_or("")
                );
                let descriptor_terms = tokenize(&descriptor_text)
                    .into_iter()
                    .collect::<BTreeSet<_>>();
                let matched_terms = task_terms
                    .iter()
                    .filter(|term| descriptor_terms.contains(*term))
                    .cloned()
                    .collect::<Vec<_>>();
                if matched_terms.is_empty() {
                    return None;
                }
                let score = u32::try_from(matched_terms.len()).unwrap_or(u32::MAX);
                let why_selected = format!(
                    "selected because task terms matched tool metadata: {}",
                    matched_terms.join(", ")
                );
                Some(ToolRecommendation {
                    tool,
                    matched_terms,
                    score,
                    why_selected,
                })
            })
            .collect::<Vec<_>>();
        recommendations.sort_by_key(|recommendation| {
            (
                Reverse(recommendation.score),
                recommendation.tool.cell_id,
                recommendation.tool.descriptor.name.clone(),
            )
        });
        recommendations.truncate(limit);
        recommendations
    }
}

impl ToolPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Execute => "execute",
            Self::Write => "write",
            Self::ApprovalRequired => "approval_required",
        }
    }
}

impl std::str::FromStr for ToolPermission {
    type Err = EngineError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "read" => Ok(Self::Read),
            "execute" => Ok(Self::Execute),
            "write" => Ok(Self::Write),
            "approval_required" => Ok(Self::ApprovalRequired),
            other => Err(invalid_tool(format!("unknown tool permission: {other}"))),
        }
    }
}

fn parse_tool_fields(body: &str) -> std::collections::BTreeMap<String, String> {
    let mut fields = std::collections::BTreeMap::new();
    for line in body.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim().to_lowercase(), value.trim().to_owned());
        }
    }
    fields
}

fn parse_permissions(value: &str) -> EngineResult<BTreeSet<ToolPermission>> {
    value
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(str::parse)
        .collect()
}

fn encode_permissions(permissions: &BTreeSet<ToolPermission>) -> String {
    permissions
        .iter()
        .map(ToolPermission::as_str)
        .collect::<Vec<_>>()
        .join(",")
}

fn optional_line(key: &str, value: Option<&str>) -> Option<String> {
    value.map(|value| format!("{key}={}", line_value(value)))
}

fn line_value(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

fn invalid_tool(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(format!("invalid tool descriptor: {}", message.into()))
}
