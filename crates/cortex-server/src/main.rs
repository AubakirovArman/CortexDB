use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    let [_, root, addr] = args.as_slice() else {
        eprintln!("usage: cortex-server <path> <addr>");
        return ExitCode::FAILURE;
    };
    match cortex_server::serve(&PathBuf::from(root), addr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
