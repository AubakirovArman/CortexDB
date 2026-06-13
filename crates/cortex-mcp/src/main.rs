use std::io::{self, BufReader, BufWriter};

use cortex_mcp::{run_stdio, McpConfig, SdkToolExecutor};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = McpConfig::from_env_and_args(std::env::args())?;
    if config.print_help {
        println!("{}", McpConfig::help());
        return Ok(());
    }
    let executor = SdkToolExecutor::from_config(config);
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio(
        BufReader::new(stdin.lock()),
        BufWriter::new(stdout.lock()),
        executor,
    )
    .map_err(|error| format!("mcp stdio failed: {error}"))
}
