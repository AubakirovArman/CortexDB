use std::time::Instant;

use crate::helpers::{document_total, round_ms};
use crate::ingest::ingest_documents;

use super::PipelineState;

pub(super) fn run(state: &mut PipelineState) -> Result<(), String> {
    if state.use_source_payload_prefilter {
        state
            .logger
            .log("skip corpus ingest; source-payload prefilter reads bounded documents directly");
        state.report_metrics.documents_indexed = 0;
        state.logger.status(
            "ingest",
            "skipped",
            "source-payload prefilter skips corpus ingest",
            Some(0),
            Some(0),
            &[],
        );
    } else if !state.args.skip_ingest {
        state.logger.log("begin corpus ingest");
        state.logger.status(
            "ingest",
            "running",
            "begin corpus ingest",
            Some(0),
            Some(document_total(&state.args, state.uuid_index.len())),
            &[],
        );
        let ingest_started = Instant::now();
        let indexed = ingest_documents(
            &mut state.db,
            &state.uuid_index,
            &mut state.document_vectors,
            state.ingest_lexical.as_mut(),
            &state.args,
            &state.logger,
        )?;
        state.report_metrics.documents_indexed = indexed;
        state.report_metrics.ingest_duration_ms =
            Some(round_ms(ingest_started.elapsed().as_secs_f64() * 1000.0));
        state.logger.log(&format!("ingested {indexed} documents"));
        state.logger.status(
            "ingest",
            "done",
            "corpus ingest done",
            Some(indexed),
            Some(document_total(&state.args, state.uuid_index.len())),
            &[],
        );
        checkpoint_if_needed(state, indexed)?;
    } else {
        state.logger.log("skip corpus ingest");
        state.report_metrics.documents_indexed = 0;
        state
            .logger
            .status("ingest", "skipped", "skip corpus ingest", None, None, &[]);
    }
    Ok(())
}

fn checkpoint_if_needed(state: &mut PipelineState, indexed: usize) -> Result<(), String> {
    if state.args.skip_checkpoint {
        state.logger.log("skip checkpoint");
        state
            .logger
            .status("checkpoint", "skipped", "skip checkpoint", None, None, &[]);
        return Ok(());
    }
    state.logger.log("begin checkpoint");
    state.logger.status(
        "checkpoint",
        "running",
        "checkpoint benchmark corpus",
        None,
        None,
        &[],
    );
    let checkpoint_started = Instant::now();
    state
        .db
        .checkpoint()
        .map_err(|error| format!("failed to checkpoint benchmark corpus: {error}"))?;
    state.report_metrics.checkpoint_duration_ms = Some(round_ms(
        checkpoint_started.elapsed().as_secs_f64() * 1000.0,
    ));
    state
        .logger
        .log(&format!("checkpointed {indexed} documents"));
    state
        .logger
        .status("checkpoint", "done", "checkpoint complete", None, None, &[]);
    Ok(())
}
