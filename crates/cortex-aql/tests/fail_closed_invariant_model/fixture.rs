use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    eval_bitmap_program, parse_aql, AqlCatalog, AqlStatement, Binder, BitmapHandle, BitmapOp,
    BrainId, CellTypeId, MemoryType, MockBitmapProvider, ScopeId, StatusId,
};

mod helpers;
use helpers::{
    agent_view, cell_type_handle, cell_type_id, scope_handle, scope_id, status_handle, status_id,
    DeterministicRng,
};

pub(crate) const MODEL_CASES: usize = 128;
const SCOPE_NAMES: [&str; 3] = ["scope-a", "scope-b", "scope-c"];
const STATUS_NAMES: [&str; 3] = ["ready", "draft", "archived"];
const TYPE_NAMES: [&str; 2] = ["fact", "document_block"];

pub(crate) fn assert_bitmap_program_model() {
    let catalog = ModelCatalog;
    let mut positive_cases = 0;
    for case_index in 0..MODEL_CASES {
        let case = ModelCase::generate(case_index as u64);
        let query = case.query();
        let AqlStatement::RetrieveContext(raw) = parse_aql(&query).unwrap() else {
            panic!("expected retrieve");
        };
        let view = agent_view(&case.readable_scopes);
        let plan = Binder::new(&catalog, &view).bind_retrieve(&raw).unwrap();

        assert_eq!(
            &plan.bitmap_program.ops[..3],
            [
                BitmapOp::PushAgentAllowed,
                BitmapOp::PushLive,
                BitmapOp::And
            ]
        );
        assert!(
            plan.bitmap_program.ops[3..]
                .iter()
                .filter(|op| matches!(op, BitmapOp::And))
                .count()
                >= case.where_predicate_count()
        );

        let provider = case.provider();
        let admitted = eval_bitmap_program(&plan.bitmap_program, &provider).unwrap();
        let spec = case.spec_admitted_candidates();
        assert!(
            admitted.is_subset(&spec),
            "case {case_index} admitted {admitted:?} outside model {spec:?}"
        );
        assert_eq!(
            admitted, spec,
            "case {case_index} bitmap program diverged from fail-closed model"
        );
        if !admitted.is_empty() {
            positive_cases += 1;
        }
    }
    assert!(
        positive_cases >= MODEL_CASES / 3,
        "property harness must include positive admitted cases; saw {positive_cases}"
    );
}

struct ModelCatalog;

impl AqlCatalog for ModelCatalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        (name == "brain").then_some(BrainId(1))
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
        SCOPE_NAMES
            .iter()
            .position(|scope| *scope == name)
            .map(scope_id)
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        STATUS_NAMES
            .iter()
            .position(|name| *name == status)
            .map(status_id)
    }

    fn resolve_cell_type(&self, _brain: BrainId, cell_type: &str) -> Option<CellTypeId> {
        TYPE_NAMES
            .iter()
            .position(|name| *name == cell_type)
            .map(cell_type_id)
    }

    fn scope_bitmap(&self, _brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        (10..13)
            .contains(&scope.0)
            .then_some(scope_handle((scope.0 - 10) as usize))
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        (20..23)
            .contains(&status.0)
            .then_some(status_handle((status.0 - 20) as usize))
    }

    fn cell_type_bitmap(&self, _brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        (30..32)
            .contains(&cell_type.0)
            .then_some(cell_type_handle((cell_type.0 - 30) as usize))
    }

    fn memory_type_bitmap(
        &self,
        _brain: BrainId,
        _memory_type: MemoryType,
    ) -> Option<BitmapHandle> {
        None
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(field, "space" | "status" | "type")
    }
}

#[derive(Clone)]
struct CellModel {
    candidate: u32,
    scope_index: usize,
    status_index: usize,
    type_index: usize,
    live: bool,
}

struct ModelCase {
    cells: Vec<CellModel>,
    readable_scopes: BTreeSet<usize>,
    where_scope: Option<usize>,
    where_status: Option<usize>,
    where_type: Option<usize>,
}

impl ModelCase {
    fn generate(seed: u64) -> Self {
        let mut rng = DeterministicRng::new(0xC0DB_0000_0000_0001 ^ seed);
        let mut readable_scopes = BTreeSet::new();
        for index in 0..SCOPE_NAMES.len() {
            if rng.chance(1, 2) {
                readable_scopes.insert(index);
            }
        }
        if readable_scopes.is_empty() {
            readable_scopes.insert(rng.index(SCOPE_NAMES.len()));
        }

        let where_scope = if rng.chance(2, 3) {
            Some(
                *readable_scopes
                    .iter()
                    .nth(rng.index(readable_scopes.len()))
                    .unwrap(),
            )
        } else {
            None
        };
        let where_status = rng.chance(2, 3).then(|| rng.index(STATUS_NAMES.len()));
        let where_type = rng.chance(1, 2).then(|| rng.index(TYPE_NAMES.len()));

        let forced_scope = where_scope.unwrap_or(*readable_scopes.iter().next().unwrap());
        let forced_status = where_status.unwrap_or(rng.index(STATUS_NAMES.len()));
        let forced_type = where_type.unwrap_or(rng.index(TYPE_NAMES.len()));
        let mut cells = vec![CellModel {
            candidate: 1,
            scope_index: forced_scope,
            status_index: forced_status,
            type_index: forced_type,
            live: true,
        }];
        for candidate in 2..=36 {
            cells.push(CellModel {
                candidate,
                scope_index: rng.index(SCOPE_NAMES.len()),
                status_index: rng.index(STATUS_NAMES.len()),
                type_index: rng.index(TYPE_NAMES.len()),
                live: rng.chance(4, 5),
            });
        }
        Self {
            cells,
            readable_scopes,
            where_scope,
            where_status,
            where_type,
        }
    }

    fn query(&self) -> String {
        let mut predicates = Vec::new();
        if let Some(index) = self.where_scope {
            predicates.push(format!(r#"space = "{}""#, SCOPE_NAMES[index]));
        }
        if let Some(index) = self.where_status {
            predicates.push(format!(r#"status = "{}""#, STATUS_NAMES[index]));
        }
        if let Some(index) = self.where_type {
            predicates.push(format!(r#"type = "{}""#, TYPE_NAMES[index]));
        }
        if predicates.is_empty() {
            r#"RETRIEVE CONTEXT FOR TASK "fail closed" IN BRAIN brain LIMIT 40 CANDIDATES;"#
                .to_owned()
        } else {
            format!(
                r#"RETRIEVE CONTEXT FOR TASK "fail closed" IN BRAIN brain WHERE {} LIMIT 40 CANDIDATES;"#,
                predicates.join(" AND ")
            )
        }
    }

    fn where_predicate_count(&self) -> usize {
        usize::from(self.where_scope.is_some())
            + usize::from(self.where_status.is_some())
            + usize::from(self.where_type.is_some())
    }

    fn provider(&self) -> MockBitmapProvider {
        let mut bitmaps = BTreeMap::<BitmapHandle, BTreeSet<u32>>::new();
        let mut agent_allowed = BTreeSet::new();
        let mut live = BTreeSet::new();
        let mut universe = BTreeSet::new();
        for cell in &self.cells {
            universe.insert(cell.candidate);
            bitmaps
                .entry(scope_handle(cell.scope_index))
                .or_default()
                .insert(cell.candidate);
            bitmaps
                .entry(status_handle(cell.status_index))
                .or_default()
                .insert(cell.candidate);
            bitmaps
                .entry(cell_type_handle(cell.type_index))
                .or_default()
                .insert(cell.candidate);
            if self.readable_scopes.contains(&cell.scope_index) {
                agent_allowed.insert(cell.candidate);
            }
            if cell.live {
                live.insert(cell.candidate);
            }
        }
        MockBitmapProvider {
            bitmaps,
            agent_allowed,
            live,
            universe,
        }
    }

    fn spec_admitted_candidates(&self) -> BTreeSet<u32> {
        self.cells
            .iter()
            .filter(|cell| self.readable_scopes.contains(&cell.scope_index))
            .filter(|cell| cell.live)
            .filter(|cell| {
                self.where_scope
                    .is_none_or(|scope| cell.scope_index == scope)
            })
            .filter(|cell| {
                self.where_status
                    .is_none_or(|status| cell.status_index == status)
            })
            .filter(|cell| {
                self.where_type
                    .is_none_or(|cell_type| cell.type_index == cell_type)
            })
            .map(|cell| cell.candidate)
            .collect()
    }
}
