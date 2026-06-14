use crate::database::{CandidateResolver, Database};
use crate::exec::pack::ExplainCollector;
use crate::exec::trace::{drain, elapsed_nanos, MaterializedOp, PhysicalOp};
use crate::feedback::current_unix_seconds;

pub(super) fn apply_memory_lifecycle_filter<P: CandidateResolver>(
    database: &Database,
    provider: &P,
    candidates: Vec<u32>,
    collector: &mut ExplainCollector,
) -> Vec<u32> {
    let input_count = candidates.len();
    let now = current_unix_seconds();
    let started = std::time::Instant::now();
    let filtered = candidates
        .into_iter()
        .filter(|candidate| {
            provider
                .cell_id_for_candidate(*candidate)
                .map(|cell_id| database.memory_lifecycle_store.is_active_at(cell_id, now))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let mut op = MaterializedOp::new(
        "MemoryLifecycleFilter",
        input_count,
        filtered,
        elapsed_nanos(started),
    );
    let candidates = drain(&mut op);
    collector.push(op.trace());
    candidates
}
