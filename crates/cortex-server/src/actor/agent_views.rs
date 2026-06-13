use std::sync::atomic::Ordering;

use cortex_aql::AgentId;

use crate::auth_agent_admin::{
    create_agent_view, list_agent_views, show_agent_view, AgentViewCreateRequest,
    AgentViewListResponse, AgentViewResponse,
};
use crate::responses::RouterError;

use super::DatabaseActor;

impl DatabaseActor {
    pub(crate) fn create_agent_view(
        &self,
        request: AgentViewCreateRequest,
    ) -> Result<AgentViewResponse, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = create_agent_view(&mut guard, request);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub(crate) fn list_agent_views(&self) -> Result<AgentViewListResponse, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let guard = self.db.read();
        let result = list_agent_views(&guard);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub(crate) fn show_agent_view(
        &self,
        agent_id: AgentId,
    ) -> Result<AgentViewResponse, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let guard = self.db.read();
        let result = show_agent_view(&guard, agent_id);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }
}
