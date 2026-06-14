use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    parse_aql, AgentId, AgentView, AqlCatalog, AqlStatement, Binder, BitmapHandle, BitmapProvider,
    BrainId, CellTypeId, MemoryType, MockBitmapProvider, RetrievalMode, ScopeId, StatusId,
    Q16_ZERO,
};
use cortex_core::CellId;
use cortex_engine::{CandidateResolver, Database};

#[derive(Default)]
struct Catalog {
    brains: BTreeMap<String, BrainId>,
    status_bitmaps: BTreeMap<StatusId, BitmapHandle>,
}

struct Provider {
    bitmap: MockBitmapProvider,
    candidate_to_cell: BTreeMap<u32, CellId>,
}

impl AqlCatalog for Catalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        self.brains.get(name).copied()
    }

    fn resolve_scope(&self, _brain: BrainId, _name: &str) -> Option<ScopeId> {
        None
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        (status == "ready").then_some(StatusId(1))
    }

    fn resolve_cell_type(&self, _brain: BrainId, _cell_type: &str) -> Option<CellTypeId> {
        None
    }

    fn scope_bitmap(&self, _brain: BrainId, _scope: ScopeId) -> Option<BitmapHandle> {
        None
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        self.status_bitmaps.get(&status).copied()
    }

    fn cell_type_bitmap(&self, _brain: BrainId, _cell_type: CellTypeId) -> Option<BitmapHandle> {
        None
    }

    fn memory_type_bitmap(
        &self,
        _brain: BrainId,
        _memory_type: MemoryType,
    ) -> Option<BitmapHandle> {
        None
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        field == "status"
    }
}

#[test]
fn bound_aql_retrieve_reads_payloads_from_database() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"ready payload".to_vec()).unwrap();
    db.put_cell(CellId(2), b"other payload".to_vec()).unwrap();

    let query = r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
WHERE status = "ready" LIMIT 10 CANDIDATES;"#;
    let AqlStatement::RetrieveContext(raw) = parse_aql(query).unwrap() else {
        panic!("expected retrieve");
    };
    let catalog = Catalog {
        brains: BTreeMap::from([("brain".to_owned(), BrainId(1))]),
        status_bitmaps: BTreeMap::from([(StatusId(1), BitmapHandle(200))]),
    };
    let view = view();
    let plan = Binder::new(&catalog, &view).bind_retrieve(&raw).unwrap();
    let provider = Provider {
        bitmap: MockBitmapProvider {
            universe: BTreeSet::from([7, 8, 9]),
            agent_allowed: BTreeSet::from([7, 8, 9]),
            live: BTreeSet::from([7, 8, 9]),
            bitmaps: BTreeMap::from([(BitmapHandle(200), BTreeSet::from([7]))]),
        },
        candidate_to_cell: BTreeMap::from([(7, CellId(1)), (8, CellId(2))]),
    };

    let cells = db.retrieve_cells(&plan, &provider).unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(1));
    assert_eq!(cells[0].payload, b"ready payload");
}

impl BitmapProvider for Provider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<cortex_aql::RoaringBitmap> {
        self.bitmap.bitmap(handle)
    }

    fn agent_allowed(&self) -> cortex_aql::RoaringBitmap {
        self.bitmap.agent_allowed()
    }

    fn live(&self) -> cortex_aql::RoaringBitmap {
        self.bitmap.live()
    }

    fn universe(&self) -> cortex_aql::RoaringBitmap {
        self.bitmap.universe()
    }
}

impl CandidateResolver for Provider {
    fn cell_id_for_candidate(&self, candidate: u32) -> Option<CellId> {
        self.candidate_to_cell.get(&candidate).copied()
    }
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::new(),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
