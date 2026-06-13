pub(crate) use super::run;

#[path = "tests/agent.rs"]
mod agent;
#[path = "tests/backup.rs"]
mod backup;
#[path = "tests/basics.rs"]
mod basics;
#[path = "tests/helpers.rs"]
mod helpers;
#[path = "tests/maintenance.rs"]
mod maintenance;
#[path = "tests/migration.rs"]
mod migration;
#[path = "tests/retrieval_core.rs"]
mod retrieval_core;
#[path = "tests/search.rs"]
mod search;
