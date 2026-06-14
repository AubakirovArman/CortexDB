use cortex_aql::AgentView;
use cortex_core::{CellDescriptor, CellId, KnowledgeCellType};

use crate::plan::PolicyRewrite;
use crate::query::scope_id;

use super::super::payload::parse_session_cell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionIndexCell {
    session_id: String,
    expires_at_unix_seconds: u64,
    descriptor: CellDescriptor,
    resident_payload: Option<Vec<u8>>,
}

impl SessionIndexCell {
    pub(super) fn from_version(
        _cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
        store_payload: bool,
    ) -> Option<Self> {
        let metadata = SessionMetadata::from_descriptor(descriptor)
            .or_else(|| SessionMetadata::from_payload(payload))?;
        Some(Self {
            session_id: metadata.session_id,
            expires_at_unix_seconds: metadata.expires_at_unix_seconds,
            descriptor: descriptor.clone(),
            resident_payload: store_payload.then(|| payload.to_vec()),
        })
    }

    pub(super) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(super) fn descriptor(&self) -> &CellDescriptor {
        &self.descriptor
    }

    pub(super) fn resident_payload(&self) -> Option<&[u8]> {
        self.resident_payload.as_deref()
    }

    pub(super) fn is_visible_to(&self, view: &AgentView, now_unix_seconds: u64) -> bool {
        now_unix_seconds < self.expires_at_unix_seconds
            && PolicyRewrite::allows_scope(view, scope_id(&self.descriptor.scope))
    }
}

struct SessionMetadata {
    session_id: String,
    expires_at_unix_seconds: u64,
}

impl SessionMetadata {
    fn from_descriptor(descriptor: &CellDescriptor) -> Option<Self> {
        if descriptor.cell_type != KnowledgeCellType::Memory {
            return None;
        }
        let session_id = descriptor.session_id.clone()?;
        descriptor.session_kind.as_ref()?;
        let ttl_seconds = descriptor.ttl_seconds?;
        let created_unix_seconds = descriptor.created_unix_seconds?;
        Some(Self {
            session_id,
            expires_at_unix_seconds: created_unix_seconds.saturating_add(ttl_seconds),
        })
    }

    fn from_payload(payload: &[u8]) -> Option<Self> {
        let metadata = parse_session_cell(payload)?;
        let expires_at_unix_seconds = metadata.expires_at_unix_seconds();
        Some(Self {
            session_id: metadata.session_id,
            expires_at_unix_seconds,
        })
    }
}
