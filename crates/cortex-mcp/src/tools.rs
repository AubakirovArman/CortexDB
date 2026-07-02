use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const TOOL_RETRIEVE_CONTEXT: &str = "retrieve_context";
pub const TOOL_VERIFY_FACT: &str = "verify_fact";
pub const TOOL_REMEMBER: &str = "remember";
pub const TOOL_SEARCH: &str = "search";

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
                "description": "Keyword-search CortexDB for cells in a scope (permission-scoped). Returns ranked cells, not a governed ContextPack; prefer retrieve_context for grounded agent context.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {"type": "string"},
                        "scope": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                }
            }
        ]
    })
}
