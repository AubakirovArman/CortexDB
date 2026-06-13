use std::env;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:8181";
const DEFAULT_TENANT: &str = "default";
const DEFAULT_SCOPE: &str = "default";
const DEFAULT_BRAIN: &str = "default";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpConfig {
    pub base_url: String,
    pub auth_token: Option<String>,
    pub tenant: String,
    pub default_scope: String,
    pub default_brain: String,
    pub print_help: bool,
}

impl McpConfig {
    pub fn from_env_and_args<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut config = Self::from_env();
        let mut args = args.into_iter().map(Into::into);
        let _program = args.next();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => config.print_help = true,
                "--base-url" => config.base_url = next_value(&mut args, "--base-url")?,
                "--auth-token" => {
                    config.auth_token = non_empty(next_value(&mut args, "--auth-token")?)
                }
                "--tenant" => config.tenant = next_value(&mut args, "--tenant")?,
                "--scope" => config.default_scope = next_value(&mut args, "--scope")?,
                "--brain" => config.default_brain = next_value(&mut args, "--brain")?,
                other => return Err(format!("unknown cortex-mcp argument: {other}")),
            }
        }
        Ok(config)
    }

    pub fn help() -> &'static str {
        "cortex-mcp\n\
         \n\
         Stdio MCP adapter for CortexDB.\n\
         \n\
         Environment:\n\
           CORTEXDB_MCP_BASE_URL     CortexDB server URL, default http://127.0.0.1:8181\n\
           CORTEXDB_MCP_AUTH_TOKEN   Bearer token; server maps it to AuthRole/AgentView\n\
           CORTEXDB_MCP_TENANT       Tenant route, default default\n\
           CORTEXDB_MCP_SCOPE        Default AgentView scope, default default\n\
           CORTEXDB_MCP_BRAIN        Default AQL brain, default default\n\
         \n\
         Flags override environment:\n\
           --base-url URL --auth-token TOKEN --tenant ID --scope SCOPE --brain BRAIN"
    }

    fn from_env() -> Self {
        Self {
            base_url: env_value("CORTEXDB_MCP_BASE_URL")
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned()),
            auth_token: env_value("CORTEXDB_MCP_AUTH_TOKEN"),
            tenant: env_value("CORTEXDB_MCP_TENANT").unwrap_or_else(|| DEFAULT_TENANT.to_owned()),
            default_scope: env_value("CORTEXDB_MCP_SCOPE")
                .unwrap_or_else(|| DEFAULT_SCOPE.to_owned()),
            default_brain: env_value("CORTEXDB_MCP_BRAIN")
                .unwrap_or_else(|| DEFAULT_BRAIN.to_owned()),
            print_help: false,
        }
    }
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().and_then(non_empty)
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
        .and_then(|value| {
            non_empty(value).ok_or_else(|| format!("{flag} requires a non-empty value"))
        })
}

#[cfg(test)]
mod tests {
    use super::McpConfig;

    #[test]
    fn parses_cli_overrides() {
        let config = McpConfig::from_env_and_args([
            "cortex-mcp",
            "--base-url",
            "http://localhost:9000",
            "--auth-token",
            "secret",
            "--tenant",
            "tenant-a",
            "--scope",
            "project:alpha",
            "--brain",
            "alpha",
        ])
        .unwrap();
        assert_eq!(config.base_url, "http://localhost:9000");
        assert_eq!(config.auth_token.as_deref(), Some("secret"));
        assert_eq!(config.tenant, "tenant-a");
        assert_eq!(config.default_scope, "project:alpha");
        assert_eq!(config.default_brain, "alpha");
    }
}
