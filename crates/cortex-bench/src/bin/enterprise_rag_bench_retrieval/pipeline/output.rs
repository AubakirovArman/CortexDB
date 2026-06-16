use std::time::Instant;

use serde_json::{json, Value};

use crate::helpers::round_ms;
use crate::io::{write_json, write_jsonl};
use crate::reporting::report_payload;

use super::PipelineState;

pub(super) fn write(
    state: &mut PipelineState,
    started: Instant,
    rows: &[Value],
) -> Result<(), String> {
    state
        .logger
        .log(&format!("write retrieval {}", state.args.output.display()));
    state.logger.status(
        "write_outputs",
        "running",
        "write retrieval output",
        None,
        None,
        &[],
    );
    write_jsonl(&state.args.output, rows)?;
    if let Some(report) = &state.args.report {
        state
            .logger
            .log(&format!("write report {}", report.display()));
        state.report_metrics.total_duration_ms = round_ms(started.elapsed().as_secs_f64() * 1000.0);
        write_json(
            report,
            &report_payload(
                &state.questions,
                &state.uuid_index,
                &state.args,
                &state.report_metrics,
            ),
        )?;
    }
    state.logger.log("finished enterprise_rag_bench_retrieval");
    state.logger.status(
        "write_outputs",
        "done",
        "retrieval output written",
        Some(rows.len()),
        Some(state.questions.len()),
        &[
            ("output", json!(state.args.output.display().to_string())),
            (
                "report",
                json!(state
                    .args
                    .report
                    .as_ref()
                    .map(|path| path.display().to_string())),
            ),
        ],
    );
    Ok(())
}
