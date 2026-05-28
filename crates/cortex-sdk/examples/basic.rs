//! Basic SDK usage example.
//!
//! Run with:
//! ```bash
//! cargo run --example basic
//! ```
//!
//! Requires a running cortex-server on 127.0.0.1:8181.

use cortex_sdk::CortexDbClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CortexDbClient::new("http://127.0.0.1:8181");

    // Health check
    let health = client.health_response()?;
    println!("Server version: {}", health.server_version);

    // Put a cell
    let put = client.put_cell_response(
        1,
        "scope=default\nstatus=ready\ntype=fact\nsource=example\n\nhello world",
    )?;
    println!("Put cell seq={} cell_id={}", put.seq, put.cell_id);

    // Read it back
    let lookup = client.get_cell_response(1)?;
    if let Some(cell) = lookup.cell {
        println!("Cell payload length: {}", cell.payload.len());
    }

    // Search
    let search = client.search_keyword_response("default", "hello", 10)?;
    println!("Search returned {} results", search.results.len());

    // Stats
    let stats = client.stats_response()?;
    println!("Current seq: {}", stats.current_seq);

    Ok(())
}
