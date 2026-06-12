mod agent;
mod backup;
mod common;
mod core;
mod maintenance;
mod search;

pub use agent::{context, forget, remember, verify};
pub use backup::{
    backup, backup_drill, backup_encrypted, backup_offsite_stage, backup_prune, restore,
    restore_encrypted,
};
pub(crate) use common::{fmt_engine_error, open_database, validate_tenant_id};
pub use core::{
    ann_validate, compact, doctor, flush, get, put, repair, run_demo, stats, tombstone, validate,
    vector_rebuild,
};
pub use maintenance::{
    gc_retired, manifest_dump, manifest_validate, unlock, wal_dump, wal_truncate, wal_validate,
};
pub(crate) use search::resolve_no_fallback_profile;
pub use search::{
    hnsw_no_fallback_profile_clear, hnsw_no_fallback_profile_set, hnsw_no_fallback_profile_show,
    search, search_explain, search_vector, SearchVectorOptions,
};
