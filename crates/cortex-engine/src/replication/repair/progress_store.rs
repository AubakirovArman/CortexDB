use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};
use crate::replication::{ConsensusState, LogIndex};

use super::worker::ReplicationRepairProgressSource;
use super::ReplicationFollowerProgress;

const MAGIC: &str = "CORTEXDB_REPLICATION_PROGRESS_V1";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationFollowerProgressStore {
    path: PathBuf,
    progress: BTreeMap<NodeId, ReplicationFollowerProgress>,
}

impl ReplicationFollowerProgressStore {
    pub fn open(path: impl AsRef<Path>) -> EngineResult<Self> {
        let path = path.as_ref().to_owned();
        let progress = if path.exists() {
            decode_progress(&fs::read_to_string(&path)?)?
        } else {
            BTreeMap::new()
        };
        Ok(Self { path, progress })
    }

    pub fn record(&mut self, progress: ReplicationFollowerProgress) -> EngineResult<()> {
        validate_progress(progress)?;
        let mut next = self.progress.clone();
        next.insert(progress.node, progress);
        write_progress(&self.path, &next)?;
        self.progress = next;
        Ok(())
    }

    pub fn record_many(
        &mut self,
        records: impl IntoIterator<Item = ReplicationFollowerProgress>,
    ) -> EngineResult<()> {
        let mut next = self.progress.clone();
        for progress in records {
            validate_progress(progress)?;
            next.insert(progress.node, progress);
        }
        write_progress(&self.path, &next)?;
        self.progress = next;
        Ok(())
    }

    pub fn progress(&self) -> &BTreeMap<NodeId, ReplicationFollowerProgress> {
        &self.progress
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug)]
pub struct ReplicationStoredProgressSource {
    store: Arc<RwLock<ReplicationFollowerProgressStore>>,
}

impl ReplicationStoredProgressSource {
    pub fn new(store: Arc<RwLock<ReplicationFollowerProgressStore>>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &Arc<RwLock<ReplicationFollowerProgressStore>> {
        &self.store
    }
}

impl<T> ReplicationRepairProgressSource<T> for ReplicationStoredProgressSource {
    fn follower_progress(
        &mut self,
        _leader: &ConsensusState,
        _transport: &T,
    ) -> EngineResult<BTreeMap<NodeId, ReplicationFollowerProgress>> {
        let store = self
            .store
            .read()
            .map_err(|_| EngineError::InvalidOperation)?;
        Ok(store.progress().clone())
    }
}

fn validate_progress(progress: ReplicationFollowerProgress) -> EngineResult<()> {
    if progress.last_observed_index < progress.commit_index {
        return Err(EngineError::InvalidOperation);
    }
    Ok(())
}

fn encode_progress(progress: &BTreeMap<NodeId, ReplicationFollowerProgress>) -> String {
    let mut out = String::from(MAGIC);
    out.push('\n');
    for item in progress.values() {
        out.push_str(&format!(
            "{} {} {}\n",
            item.node.0, item.commit_index.0, item.last_observed_index.0
        ));
    }
    out
}

fn write_progress(
    path: &Path,
    progress: &BTreeMap<NodeId, ReplicationFollowerProgress>,
) -> EngineResult<()> {
    write_atomic(path, encode_progress(progress).as_bytes())
}

fn decode_progress(input: &str) -> EngineResult<BTreeMap<NodeId, ReplicationFollowerProgress>> {
    let mut lines = input.lines();
    if lines.next() != Some(MAGIC) {
        return Err(EngineError::InvalidOperation);
    }

    let mut progress = BTreeMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let node = parse_u64(fields.next())?;
        let commit_index = parse_u64(fields.next())?;
        let last_observed_index = parse_u64(fields.next())?;
        if fields.next().is_some() {
            return Err(EngineError::InvalidOperation);
        }
        let item = ReplicationFollowerProgress::new(
            NodeId(node),
            LogIndex(commit_index),
            LogIndex(last_observed_index),
        );
        validate_progress(item)?;
        if progress.insert(item.node, item).is_some() {
            return Err(EngineError::InvalidOperation);
        }
    }
    Ok(progress)
}

fn parse_u64(value: Option<&str>) -> EngineResult<u64> {
    value
        .ok_or(EngineError::InvalidOperation)?
        .parse()
        .map_err(|_| EngineError::InvalidOperation)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> EngineResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path);
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        OpenOptions::new().read(true).open(parent)?.sync_all()?;
    }
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("replication-progress");
    path.with_file_name(format!(
        "{file_name}.tmp.{}.{}",
        std::process::id(),
        counter
    ))
}
