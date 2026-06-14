mod aql;
mod client;
mod config;

pub(crate) use aql::{
    aql_task_has_query_vector, inject_query_vector, retrieve_task_text,
    semantic_aql_needs_query_vector,
};
pub(crate) use client::format_vector_literal;
pub(crate) use config::{embed_query_from_env, missing_vector_or_config_error, parse_bool_param};

pub(crate) const MISSING_VECTOR_OR_CONFIG: &str = "semantic requires vector or embedding config";
