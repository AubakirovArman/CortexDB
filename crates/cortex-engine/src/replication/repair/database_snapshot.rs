use std::sync::{Arc, RwLock};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::replication::{
    ReplicationRepairSnapshotSource, ReplicationSnapshotRepairRequest, SnapshotSegment,
};

#[derive(Clone)]
pub struct ReplicationDatabaseSnapshotSource {
    database: Arc<RwLock<Database>>,
}

impl ReplicationDatabaseSnapshotSource {
    pub fn new(database: Arc<RwLock<Database>>) -> Self {
        Self { database }
    }
}

impl ReplicationRepairSnapshotSource for ReplicationDatabaseSnapshotSource {
    fn snapshot_for_repair(
        &mut self,
        request: &ReplicationSnapshotRepairRequest,
    ) -> EngineResult<Option<SnapshotSegment>> {
        let database = self
            .database
            .read()
            .map_err(|_| EngineError::InvalidOperation)?;
        let snapshot = database.replication_snapshot_segment()?;
        if snapshot.checkpoint_seq.0 == request.checkpoint.0 {
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }
}
