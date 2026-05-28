use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = env::args().collect::<Vec<_>>();
    let [_, root, addr] = args.as_slice() else {
        eprintln!("usage: cortex-server <path> <addr>");
        return ExitCode::FAILURE;
    };
    let options = cortex_server::ServerOptions {
        auth_token: env::var("CORTEXDB_AUTH_TOKEN").ok(),
        actor_queue_capacity: 1024,
    };
    match cortex_server::serve_with_options(&PathBuf::from(root), addr, options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
