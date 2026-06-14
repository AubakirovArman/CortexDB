use std::env;
use std::fs;
use std::time::Instant;

use cortex_engine::{ContextPackOptions, Database, DatabaseOptions, SearchLimit};

use super::args::Args;
use super::metrics::{measure_once, measure_repeated, memory_phase, sampled_cell_id};
use super::workload::{ingest_batches, scale_view};
use prepared::prepare_direct_checkpoint;
use report::write_report;

#[path = "prepared.rs"]
mod prepared;
#[path = "report.rs"]
mod report;

const VERIFY_AQL: &str =
    r#"VERIFY FACT "scale target onboarding budget approved" IN BRAIN default;"#;
const CONTEXT_AQL: &str = r#"RETRIEVE CONTEXT FOR TASK "onboarding latency budget risk" IN BRAIN default WHERE space = scale AND status = "ready" LIMIT 10 CANDIDATES;"#;

pub(crate) fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1))?;
    let started = Instant::now();
    let db_path = args.root.join("db");
    let mut phases = Vec::new();
    let view = scale_view();
    let options = DatabaseOptions {
        payload_residency: args.payload_residency,
        rebuild_lazy_payload_indexes_on_open: args.payload_residency
            != cortex_engine::PayloadResidency::Lazy,
        ..DatabaseOptions::default()
    };

    if args.reopen_only {
        if !db_path.exists() {
            return Err(format!(
                "--reopen-only requires an existing database at {}",
                db_path.display()
            ));
        }
    } else {
        fs::remove_dir_all(&args.root).ok();
        fs::create_dir_all(&args.root)
            .map_err(|error| format!("failed to create {}: {error}", args.root.display()))?;
        if args.direct_checkpoint {
            eprintln!("[scale-bench] direct_checkpoint cells={}", args.cells);
            phases.push(prepare_direct_checkpoint(&db_path, &args)?);
        }
    }

    let open_phase = if args.reopen_only || args.direct_checkpoint {
        "open_prepared"
    } else {
        "open_empty"
    };
    eprintln!("[scale-bench] {open_phase}");
    let (mut db, phase) = measure_once(open_phase, 1, || {
        Database::open_with_options(&db_path, options)
    })?;
    phases.push(phase);
    if args.direct_checkpoint || args.reopen_only {
        eprintln!("[scale-bench] memory after_open_prepared");
        phases.push(memory_phase(
            "after_open_prepared",
            &db,
            args.skip_storage_estimates,
        )?);
    } else {
        eprintln!("[scale-bench] put_batches cells={}", args.cells);
        phases.push(ingest_batches(&mut db, &args)?);
        eprintln!("[scale-bench] memory after_put");
        phases.push(memory_phase("after_put", &db, args.skip_storage_estimates)?);
        eprintln!("[scale-bench] checkpoint");
        phases.push(
            measure_once("checkpoint", args.cells, || {
                db.checkpoint()
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            })?
            .1,
        );
        eprintln!("[scale-bench] memory after_checkpoint");
        phases.push(memory_phase(
            "after_checkpoint",
            &db,
            args.skip_storage_estimates,
        )?);
    }

    if args.samples > 0 {
        eprintln!("[scale-bench] get_latest samples={}", args.samples);
        phases.push(measure_repeated("get_latest", args.samples, |offset| {
            let cell_id = sampled_cell_id(args.cells, offset, args.samples);
            let payload = db
                .get_latest_cell(cell_id)
                .ok_or_else(|| format!("missing sampled cell {}", cell_id.0))?;
            if payload.is_empty() {
                return Err(format!("empty sampled cell {}", cell_id.0));
            }
            Ok(())
        })?);
    }

    if args.search_samples > 0 {
        eprintln!(
            "[scale-bench] keyword_search samples={}",
            args.search_samples
        );
        phases.push(measure_repeated(
            "keyword_search",
            args.search_samples,
            |_| {
                let results = db
                    .search_keyword("onboarding latency budget risk", &view, SearchLimit(10))
                    .map_err(|error| error.to_string())?;
                if results.is_empty() {
                    return Err("keyword search returned no results".to_owned());
                }
                Ok(())
            },
        )?);
    }

    if args.context_samples > 0 {
        eprintln!(
            "[scale-bench] context_pack samples={}",
            args.context_samples
        );
        phases.push(measure_repeated(
            "context_pack",
            args.context_samples,
            |_| {
                let pack = db
                    .context_pack_from_aql(CONTEXT_AQL, &view, ContextPackOptions::default())
                    .map_err(|error| error.to_string())?;
                if pack.cells.is_empty() {
                    return Err("ContextPack returned no cells".to_owned());
                }
                Ok(())
            },
        )?);
    }

    if args.verify_samples > 0 {
        eprintln!("[scale-bench] verify_fact samples={}", args.verify_samples);
        phases.push(measure_repeated(
            "verify_fact",
            args.verify_samples,
            |_| {
                let report = db
                    .verify_fact_aql(VERIFY_AQL, &view)
                    .map_err(|error| error.to_string())?;
                if report.evidence.is_empty() {
                    return Err("VERIFY FACT returned no evidence".to_owned());
                }
                Ok(())
            },
        )?);
    }

    let validation = if args.skip_validation {
        cortex_engine::validation::StorageValidationReport::default()
    } else {
        db.validate_storage_report()
    };
    let mut errors = validation.errors.clone();
    eprintln!("[scale-bench] close");
    phases.push(measure_once("close", 1, || db.close())?.1);
    eprintln!("[scale-bench] restart_open");
    let (reopened, phase) = measure_once("restart_open", args.cells, || {
        Database::open_with_options(&db_path, options)
    })?;
    phases.push(phase);
    if !args.skip_validation {
        let restart_validation = reopened.validate_storage_report();
        errors.extend(
            restart_validation
                .errors
                .iter()
                .map(|error| format!("restart: {error}")),
        );
    }
    reopened
        .close()
        .map_err(|error| format!("restart close failed: {error}"))?;
    write_report(&args, started, &phases, &validation, errors)
}
