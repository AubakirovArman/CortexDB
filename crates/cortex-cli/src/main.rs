use std::env;
use std::process::ExitCode;

use cortex_core::CellId;
use cortex_engine::Database;

fn main() -> ExitCode {
    match run(env::args().collect()) {
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
    let [_, command, path, rest @ ..] = args.as_slice() else {
        return Err(usage());
    };
    match command.as_str() {
        "put" => {
            let [cell_id, payload] = rest else {
                return Err(usage());
            };
            let cell_id = parse_cell_id(cell_id)?;
            let payload = payload.as_bytes().to_vec();
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let seq = db
                .put_cell(cell_id, payload)
                .map_err(|error| error.to_string())?;
            Ok(format!("seq={}", seq.0))
        }
        "get" => {
            let [cell_id] = rest else { return Err(usage()) };
            let cell_id = parse_cell_id(cell_id)?;
            let db = Database::open(path).map_err(|error| error.to_string())?;
            Ok(db
                .get_latest_cell(cell_id)
                .map(|payload| String::from_utf8_lossy(&payload).into_owned())
                .unwrap_or_else(|| "null".to_owned()))
        }
        "tombstone" => {
            let [cell_id] = rest else { return Err(usage()) };
            let cell_id = parse_cell_id(cell_id)?;
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let seq = db
                .tombstone_cell(cell_id)
                .map_err(|error| error.to_string())?;
            Ok(format!("seq={}", seq.0))
        }
        "flush" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let stats = db.checkpoint().map_err(|error| error.to_string())?;
            Ok(format!(
                "checkpoint_seq={} cells_flushed={}",
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        _ => Err(usage()),
    }
}

fn parse_cell_id(value: &str) -> Result<CellId, String> {
    value
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| "cell_id must be u64".to_owned())
}

fn usage() -> String {
    "usage: cortexdb put <path> <cell_id> <payload> | get <path> <cell_id> | tombstone <path> <cell_id> | flush <path>"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn usage_is_reported_for_missing_args() {
        assert!(run(vec!["cortexdb".to_owned()])
            .unwrap_err()
            .contains("usage:"));
    }
}
