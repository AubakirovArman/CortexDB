use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const TOOL_RETRIEVE_CONTEXT: &str = "retrieve_context";
pub const TOOL_VERIFY_FACT: &str = "verify_fact";
pub const TOOL_REMEMBER: &str = "remember";
pub const TOOL_SEARCH: &str = "search";
pub const TOOL_CONSOLIDATE_PLAN: &str = "consolidate_plan";
pub const TOOL_CONSOLIDATE_COMMIT: &str = "consolidate_commit";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RetrieveContextArgs {
    pub task: String,
    pub scope: Option<String>,
    pub brain: Option<String>,
    pub mode: Option<String>,
    pub budget_tokens: Option<u64>,
    pub candidate_limit: Option<u32>,
    pub where_clause: Option<String>,
    pub require_citations: Option<bool>,
    pub format: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VerifyFactArgs {
    pub fact: String,
    pub scope: Option<String>,
    pub brain: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RememberArgs {
    pub content: String,
    pub scope: Option<String>,
    pub memory_type: Option<String>,
    pub ttl_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchArgs {
    pub query: String,
    pub scope: Option<String>,
    pub limit: Option<u32>,
    pub mode: Option<String>,
}

/// F04-B4.4: step 1 of the MCP consolidation protocol — which stale episodic
/// groups in a scope to summarize.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConsolidatePlanArgs {
    pub scope: String,
    pub freshness_below_q16: u16,
    pub max_groups: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConsolidateSourceRefArg {
    pub source_cell_id: u64,
    pub source_byte_start: u64,
    pub source_byte_end: u64,
}

/// F04-B4.4: step 2 of the MCP consolidation protocol — commit the summary the
/// agent generated over the planned group's source cells.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConsolidateCommitArgs {
    pub scope: String,
    #[serde(default)]
    pub summary_cell_id: Option<u64>,
    pub summary_payload: String,
    pub source_refs: Vec<ConsolidateSourceRefArg>,
    pub answerability_q16: u16,
    pub external_worker: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ToolTextContent {
    #[serde(rename = "type")]
    pub content_type: &'static str,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ToolCallResult {
    pub content: Vec<ToolTextContent>,
    #[serde(rename = "isError", skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

impl ToolCallResult {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolTextContent {
                content_type: "text",
                text: text.into(),
            }],
            is_error: false,
        }
    }

    pub fn error_text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolTextContent {
                content_type: "text",
                text: text.into(),
            }],
            is_error: true,
        }
    }
}

pub trait ToolExecutor {
    fn retrieve_context(&self, args: RetrieveContextArgs) -> Result<ToolCallResult, String>;
    fn verify_fact(&self, args: VerifyFactArgs) -> Result<ToolCallResult, String>;
    fn remember(&self, args: RememberArgs) -> Result<ToolCallResult, String>;
    fn search(&self, args: SearchArgs) -> Result<ToolCallResult, String>;
    fn consolidate_plan(&self, args: ConsolidatePlanArgs) -> Result<ToolCallResult, String>;
    fn consolidate_commit(&self, args: ConsolidateCommitArgs) -> Result<ToolCallResult, String>;
}

pub fn tools_list_result() -> Value {
    json!({
        "tools": [
            {
                "name": TOOL_RETRIEVE_CONTEXT,
                "description": "Retrieve a CortexDB ContextPack for an agent task.",
                "inputSchema": {
                    "type": "object",
                    "required": ["task"],
                    "properties": {
                        "task": {"type": "string"},
                        "scope": {"type": "string"},
                        "brain": {"type": "string"},
                        "mode": {"type": "string", "enum": ["fast", "balanced", "semantic", "audit"]},
                        "budget_tokens": {"type": "integer", "minimum": 1},
                        "candidate_limit": {"type": "integer", "minimum": 1},
                        "where_clause": {"type": "string"},
                        "require_citations": {"type": "boolean"},
                        "format": {"type": "string", "enum": ["json", "prompt", "markdown"]}
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": TOOL_VERIFY_FACT,
                "description": "Run deterministic CortexDB VERIFY FACT over evidence.",
                "inputSchema": {
                    "type": "object",
                    "required": ["fact"],
                    "properties": {
                        "fact": {"type": "string"},
                        "scope": {"type": "string"},
                        "brain": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": TOOL_REMEMBER,
                "description": "Write an agent memory through CortexDB REMEMBER.",
                "inputSchema": {
                    "type": "object",
                    "required": ["content"],
                    "properties": {
                        "content": {"type": "string"},
                        "scope": {"type": "string"},
                        "memory_type": {"type": "string"},
                        "ttl_seconds": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": TOOL_SEARCH,
                "description": "Search CortexDB for cells in a scope (permission-scoped). Default mode is keyword. Set mode=semantic|hybrid|auto to search by meaning: the query text is embedded server-side, so no vector is needed (requires an embedding endpoint configured on the server, else it fails closed). Returns ranked cells, not a governed ContextPack; prefer retrieve_context for grounded agent context.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {"type": "string"},
                        "scope": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1},
                        "mode": {
                            "type": "string",
                            "enum": ["keyword", "semantic", "hybrid", "auto"],
                            "description": "keyword (default, lexical); semantic (embed query, vector search); hybrid (lexical + vector); auto (server picks)."
                        }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": TOOL_CONSOLIDATE_PLAN,
                "description": "Plan memory consolidation: list the stale episodic memory groups in a scope that are eligible for semantic summarization. Step 1 of the two-step consolidation protocol — the agent summarizes a returned group, then calls consolidate_commit. Requires the semantic_compression feature on the server.",
                "inputSchema": {
                    "type": "object",
                    "required": ["scope", "freshness_below_q16", "max_groups"],
                    "properties": {
                        "scope": {"type": "string"},
                        "freshness_below_q16": {"type": "integer", "minimum": 0, "maximum": 65535, "description": "Select episodic cells whose freshness (Q16) has decayed below this."},
                        "max_groups": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": TOOL_CONSOLIDATE_COMMIT,
                "description": "Commit a memory-consolidation summary over source cells. Step 2 of the two-step consolidation protocol: durably record the summary cell the agent generated plus the byte ranges of the source cells it consolidated. Requires the semantic_compression feature.",
                "inputSchema": {
                    "type": "object",
                    "required": ["scope", "summary_payload", "source_refs", "answerability_q16", "external_worker"],
                    "properties": {
                        "scope": {"type": "string"},
                        "summary_cell_id": {"type": "integer"},
                        "summary_payload": {"type": "string", "description": "The summary cell payload (line-based UTF-8, compression_kind=semantic_summary)."},
                        "source_refs": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["source_cell_id", "source_byte_start", "source_byte_end"],
                                "properties": {
                                    "source_cell_id": {"type": "integer"},
                                    "source_byte_start": {"type": "integer"},
                                    "source_byte_end": {"type": "integer"}
                                }
                            }
                        },
                        "answerability_q16": {"type": "integer", "minimum": 0, "maximum": 65535},
                        "external_worker": {"type": "string"},
                        "idempotency_key": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            }
        ]
    })
}
