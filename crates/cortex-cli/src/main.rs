use std::process::ExitCode;

mod cli;
mod cli_ann;
mod cli_audit;
mod cli_audit_chain;
mod cli_audit_siem;
#[cfg(test)]
mod cli_audit_siem_tests;
#[cfg(test)]
mod cli_audit_tests;
mod cli_ingest;
#[cfg(test)]
mod cli_ingest_tests;
mod cli_json;
mod cli_json_types;
mod cli_ops;
mod context;
#[cfg(test)]
mod json_tests;
mod manifest;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_tests;
#[cfg(test)]
mod vector_tests;
mod wal;

fn main() -> ExitCode {
    match run(std::env::args().collect()) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    cli::run(args)
}
