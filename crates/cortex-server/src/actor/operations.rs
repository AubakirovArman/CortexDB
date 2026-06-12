use std::sync::atomic::Ordering;

use cortex_aql::AgentId;
use cortex_engine::{CompactionStats, ExpiredMemoryCell};

use crate::auth_policy_cells::{self, AuthPolicyCellSyncReport};
use crate::auth_scope_admin::{
    apply_agent_scope_mutation, AgentScopeAccess, AgentScopeMutationResponse,
};
use crate::responses::RouterError;

use super::DatabaseActor;

impl DatabaseActor {
    pub fn expire_memory(
        &self,
        now_unix_seconds: u64,
    ) -> Result<Vec<ExpiredMemoryCell>, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = guard
            .expire_memory_cells(now_unix_seconds)
            .map_err(|e| RouterError::Internal(e.to_string()));
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Run an incremental compaction if storage pressure or the live segment
    /// count crosses the configured threshold. Returns `None` when no work was
    /// performed, including when foreground writes are waiting so the
    /// compaction does not starve them.
    pub fn maybe_incremental_compact(&self) -> Result<Option<CompactionStats>, RouterError> {
        self.ensure_open()?;
        if self.waiting_writers() > 0 {
            return Ok(None);
        }
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = guard
            .maybe_incremental_compact()
            .map_err(|e| RouterError::Internal(e.to_string()));
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Force an incremental compaction regardless of current pressure.
    pub fn incremental_compact(&self) -> Result<CompactionStats, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = guard
            .incremental_compact()
            .map_err(|e| RouterError::Internal(e.to_string()));
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub(crate) fn sync_auth_policy_store(
        &self,
        store_json: &str,
    ) -> Result<AuthPolicyCellSyncReport, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = auth_policy_cells::sync_store_json_to_database(&mut guard, store_json);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub(crate) fn mutate_agent_scope(
        &self,
        agent_id: AgentId,
        scope: &str,
        access: AgentScopeAccess,
        grant: bool,
    ) -> Result<AgentScopeMutationResponse, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = apply_agent_scope_mutation(&mut guard, agent_id, scope, access, grant);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }
}
