use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::error::{EngineError, EngineResult};
use crate::replication::{
    run_replication_repair_worker, ConsensusState, ReplicationRepairProgressSource,
    ReplicationRepairSnapshotSource, ReplicationRepairWorkerPolicy, ReplicationRepairWorkerReport,
    ReplicationSnapshotTransport, ReplicationTransport,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationRepairBackgroundPolicy {
    pub worker: ReplicationRepairWorkerPolicy,
    pub tick_interval: Duration,
    pub max_runs: Option<usize>,
    pub stop_when_idle: bool,
}

impl Default for ReplicationRepairBackgroundPolicy {
    fn default() -> Self {
        Self {
            worker: ReplicationRepairWorkerPolicy::default(),
            tick_interval: Duration::from_secs(1),
            max_runs: None,
            stop_when_idle: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplicationRepairBackgroundReport {
    pub runs: Vec<ReplicationRepairWorkerReport>,
    pub stopped: bool,
}

impl ReplicationRepairBackgroundReport {
    pub fn append_repairs(&self) -> usize {
        self.runs
            .iter()
            .map(ReplicationRepairWorkerReport::append_repairs)
            .sum()
    }

    pub fn snapshots_sent(&self) -> usize {
        self.runs
            .iter()
            .map(ReplicationRepairWorkerReport::snapshots_sent)
            .sum()
    }

    pub fn pending_snapshots(&self) -> usize {
        self.runs
            .last()
            .map(ReplicationRepairWorkerReport::pending_snapshots)
            .unwrap_or(0)
    }

    pub fn last_run_idle(&self) -> bool {
        self.runs
            .last()
            .map(ReplicationRepairWorkerReport::is_idle)
            .unwrap_or(true)
    }
}

pub struct ReplicationRepairBackgroundHandle {
    stop_tx: Sender<()>,
    join: JoinHandle<EngineResult<ReplicationRepairBackgroundReport>>,
}

impl ReplicationRepairBackgroundHandle {
    pub fn stop(self) -> EngineResult<ReplicationRepairBackgroundReport> {
        let _ = self.stop_tx.send(());
        self.join()
    }

    pub fn join(self) -> EngineResult<ReplicationRepairBackgroundReport> {
        self.join
            .join()
            .map_err(|_| EngineError::InvalidOperation)?
    }
}

pub fn spawn_replication_repair_background_task<T, P, S>(
    leader: ConsensusState,
    mut transport: T,
    mut progress_source: P,
    mut snapshot_source: S,
    policy: ReplicationRepairBackgroundPolicy,
) -> EngineResult<ReplicationRepairBackgroundHandle>
where
    T: ReplicationTransport + ReplicationSnapshotTransport + Send + 'static,
    P: ReplicationRepairProgressSource<T> + Send + 'static,
    S: ReplicationRepairSnapshotSource + Send + 'static,
{
    if policy.worker.max_ticks == 0 {
        return Err(EngineError::InvalidOperation);
    }
    if matches!(policy.max_runs, Some(0)) {
        return Err(EngineError::InvalidOperation);
    }

    let (stop_tx, stop_rx) = mpsc::channel();
    let join = thread::spawn(move || {
        let mut report = ReplicationRepairBackgroundReport::default();
        let mut runs = 0usize;
        loop {
            if stop_rx.try_recv().is_ok() {
                report.stopped = true;
                break;
            }
            let run = run_replication_repair_worker(
                &leader,
                &mut transport,
                &mut progress_source,
                &mut snapshot_source,
                policy.worker,
            )?;
            runs = runs.checked_add(1).ok_or(EngineError::InvalidOperation)?;
            let should_stop_idle = policy.stop_when_idle && run.is_idle();
            report.runs.push(run);
            if should_stop_idle || policy.max_runs.is_some_and(|max| runs >= max) {
                break;
            }
            match stop_rx.recv_timeout(policy.tick_interval) {
                Ok(_) => {
                    report.stopped = true;
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(report)
    });

    Ok(ReplicationRepairBackgroundHandle { stop_tx, join })
}
