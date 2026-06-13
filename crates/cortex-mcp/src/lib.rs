//! MCP stdio adapter for CortexDB.

pub mod config;
pub mod protocol;
pub mod sdk_executor;
pub mod server;
pub mod tools;

pub use config::McpConfig;
pub use sdk_executor::SdkToolExecutor;
pub use server::run_stdio;
