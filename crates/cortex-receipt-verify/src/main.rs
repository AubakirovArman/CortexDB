use std::env;
use std::fs;

use cortex_receipt_verify::{verify_input, VerifyInput};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(flag) = args.next() else {
        return Err(usage());
    };
    if flag != "--input" {
        return Err(usage());
    }
    let Some(input_path) = args.next() else {
        return Err(usage());
    };
    if args.next().is_some() {
        return Err(usage());
    }

    let text = fs::read_to_string(&input_path)
        .map_err(|error| format!("failed to read {input_path}: {error}"))?;
    let input = serde_json::from_str::<VerifyInput>(&text)
        .map_err(|error| format!("invalid verifier input JSON: {error}"))?;
    verify_input(&input).map_err(|error| format!("receipt verification failed: {error:?}"))?;
    println!("accountability receipt verified: {input_path}");
    Ok(())
}

fn usage() -> String {
    "usage: cortex-receipt-verify --input <verify-input.json>".to_owned()
}
