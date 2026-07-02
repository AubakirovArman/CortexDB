use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BoundPlan, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;

use crate::{
    scope_id, ContextPackOptions, Database, DatabaseOptions, EngineFeatureFlags, SearchMode,
    SearchQuery,
};

const ENGINE_MODEL_CASES: usize = 32;
const SCOPE_NAMES: [&str; 3] = ["scope-a", "scope-b", "scope-c"];
const STATUS_NAMES: [&str; 3] = ["ready", "draft", "archived"];
const TYPE_NAMES: [&str; 2] = ["fact", "document_block"];

#[test]
fn persisted_ann_and_lexical_paths_respect_fail_closed_model() {
    let mut positive_cases = 0;
    for case_index in 0..ENGINE_MODEL_CASES {
        let case = EngineCase::generate(case_index as u64);
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open_with_options(dir.path(), hnsw_options()).unwrap();
        case.seed_database(&mut db);
        db.checkpoint().unwrap();

        let view = agent_view(&case.readable_scopes);
        let query = case.query();
        let expected = case.expected_cell_ids();
        if !expected.is_empty() {
            positive_cases += 1;
        }

        let pack = db
            .context_pack_from_aql(&query, &view, ContextPackOptions::default())
            .unwrap();
        assert_cell_subset(
            case_index,
            "context_pack",
            pack.cells.iter().map(|cell| cell.cell_id),
            &expected,
        );

        let (cached, _) = db.bind_aql_cached(&query, &view).unwrap();
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            panic!("expected retrieve plan");
        };
        for mode in [SearchMode::Keyword, SearchMode::Vector, SearchMode::Hybrid] {
            let vector = [100, 0];
            let outcome = db
                .search_cells_with_bound_retrieve_plan(
                    SearchQuery {
                        text: "fail closed invariant probe",
                        vector: matches!(mode, SearchMode::Vector | SearchMode::Hybrid)
                            .then_some(vector.as_slice()),
                        limit: 40,
                        mode,
                    },
                    &view,
                    plan.as_ref(),
                )
                .unwrap();
            assert_cell_subset(
                case_index,
                search_mode_name(mode),
                outcome.results.iter().map(|result| result.cell_id),
                &expected,
            );
            if !expected.is_empty() && mode == SearchMode::Keyword {
                assert!(
                    !outcome.results.is_empty(),
                    "case {case_index} keyword path must have a positive admitted result"
                );
            }
        }
    }
    assert!(
        positive_cases >= ENGINE_MODEL_CASES / 2,
        "engine model must include positive admitted cases; saw {positive_cases}"
    );
}

#[derive(Clone)]
struct EngineCell {
    cell_id: CellId,
    scope_index: usize,
    status_index: usize,
    type_index: usize,
}

struct EngineCase {
    cells: Vec<EngineCell>,
    readable_scopes: BTreeSet<usize>,
    where_scope: Option<usize>,
    where_status: Option<usize>,
    where_type: Option<usize>,
}

impl EngineCase {
    fn generate(seed: u64) -> Self {
        let mut rng = DeterministicRng::new(0xFC70_0000_0000_0001 ^ seed);
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
        let mut cells = vec![EngineCell {
            cell_id: CellId(1),
            scope_index: forced_scope,
            status_index: forced_status,
            type_index: forced_type,
        }];
        for id in 2..=30 {
            cells.push(EngineCell {
                cell_id: CellId(id),
                scope_index: rng.index(SCOPE_NAMES.len()),
                status_index: rng.index(STATUS_NAMES.len()),
                type_index: rng.index(TYPE_NAMES.len()),
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

    fn seed_database(&self, db: &mut Database) {
        for cell in &self.cells {
            db.put_cell(cell.cell_id, self.payload(cell).into_bytes())
                .unwrap();
        }
    }

    fn payload(&self, cell: &EngineCell) -> String {
        format!(
            "scope={}\nstatus={}\ntype={}\nvector={},{}\n\nfail closed invariant probe cell {}",
            SCOPE_NAMES[cell.scope_index],
            STATUS_NAMES[cell.status_index],
            TYPE_NAMES[cell.type_index],
            100 - cell.cell_id.0 as i16,
            cell.cell_id.0 as i16,
            cell.cell_id.0
        )
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
            r#"RETRIEVE CONTEXT FOR TASK "fail closed invariant probe" IN BRAIN investment_projects LIMIT 40 CANDIDATES;"#.to_owned()
        } else {
            format!(
                r#"RETRIEVE CONTEXT FOR TASK "fail closed invariant probe" IN BRAIN investment_projects WHERE {} LIMIT 40 CANDIDATES;"#,
                predicates.join(" AND ")
            )
        }
    }

    fn expected_cell_ids(&self) -> BTreeSet<CellId> {
        self.cells
            .iter()
            .filter(|cell| self.readable_scopes.contains(&cell.scope_index))
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
            .map(|cell| cell.cell_id)
            .collect()
    }
}

struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn index(&mut self, upper: usize) -> usize {
        (self.next_u32() as usize) % upper
    }

    fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.next_u32() % denominator < numerator
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }
}

fn agent_view(readable_scope_indexes: &BTreeSet<usize>) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: readable_scope_indexes
            .iter()
            .map(|index| scope_id(SCOPE_NAMES[*index]))
            .collect(),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 2_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 40,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn assert_cell_subset(
    case_index: usize,
    surface: &str,
    observed: impl Iterator<Item = CellId>,
    expected: &BTreeSet<CellId>,
) {
    for cell_id in observed {
        assert!(
            expected.contains(&cell_id),
            "case {case_index} {surface} admitted {cell_id:?} outside {expected:?}"
        );
    }
}

fn search_mode_name(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Keyword => "persisted_keyword",
        SearchMode::Vector => "persisted_vector",
        SearchMode::Hybrid => "persisted_hybrid",
        _ => "other",
    }
}

fn hnsw_options() -> DatabaseOptions {
    DatabaseOptions {
        feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
        ..DatabaseOptions::default()
    }
}
