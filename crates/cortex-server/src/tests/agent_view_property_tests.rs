//! Property suite for the AgentView permission invariant.
//!
//! For every read surface we generate a random corpus with per-scope secret
//! markers, pick a random subset of scopes as readable, and assert that no
//! response body (success or error) contains a secret marker from an unreadable
//! scope. The suite runs against both the memtable path and the persisted-index
//! path (after flush).

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use cortex_aql::{AgentId, AgentView, BrainId, RetrievalMode, ScopeId};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, DatabaseOptions};

use crate::auth::{AuthRole, AuthTokenPolicy};
use crate::handle_http_with_options;
use crate::ServerOptions;

const SCOPES: &[&str] = &["alpha", "beta", "gamma", "delta"];
const BRAIN_ID: BrainId = BrainId(1);
const AGENT_ID: AgentId = AgentId(7);
const TOKEN: &str = "property-suite-token";

/// Tiny deterministic LCG.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }
}
fn random_subset(rng: &mut Rng, set: &BTreeSet<ScopeId>) -> BTreeSet<ScopeId> {
    let mut result = BTreeSet::new();
    for scope in set {
        if rng.next().is_multiple_of(2) {
            result.insert(*scope);
        }
    }
    result
}

fn build_view(readable_scopes: BTreeSet<ScopeId>) -> AgentView {
    AgentView {
        agent_id: AGENT_ID,
        label: Some("property-suite".to_owned()),
        readable_brains: BTreeSet::from([BRAIN_ID]),
        readable_scopes,
        writable_scopes: BTreeSet::new(),
        allowed_modes: [
            RetrievalMode::Fast,
            RetrievalMode::Balanced,
            RetrievalMode::Semantic,
        ]
        .into_iter()
        .collect(),
        allowed_memory_types: BTreeSet::new(),
        max_context_budget_tokens: 4000,
        default_context_budget_tokens: 1000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: 0,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn server_options(_root: &Path) -> ServerOptions {
    ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new(TOKEN, AuthRole::Data).with_agent_id(AGENT_ID.0)],
        engine_database_options: DatabaseOptions::default(),
        ..ServerOptions::default()
    }
}

fn http_request(method: &str, target: &str, body: &str) -> String {
    format!(
        "{} {} HTTP/1.1\r\nauthorization: {}\r\n\r\n{}",
        method, target, TOKEN, body
    )
}

fn response_body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map_or(response, |(_, body)| body)
}

struct Corpus {
    /// scope -> (cell_id, secret_marker)
    cells: HashMap<String, Vec<(u64, String)>>,
}

impl Corpus {
    fn secrets_for_scope(&self, scope: &str) -> Vec<String> {
        self.cells
            .get(scope)
            .map(|v| v.iter().map(|(_, secret)| secret.clone()).collect())
            .unwrap_or_default()
    }
}

fn build_corpus(db: &mut Database, _rng: &mut Rng) -> Corpus {
    let mut cells: HashMap<String, Vec<(u64, String)>> = HashMap::new();
    let mut next_cell_id: u64 = 1000;
    for scope in SCOPES {
        let mut scope_cells = Vec::new();
        for index in 0..3 {
            let cell_id = next_cell_id;
            next_cell_id += 1;
            let secret = format!("SECRET_{scope}_{index}");
            let payload = format!(
                "scope={scope}\nstatus=ready\nsource_trust_q16=50000\n\ncommon content {secret}"
            );
            db.put_cell(CellId(cell_id), payload.into_bytes()).unwrap();
            scope_cells.push((cell_id, secret));
        }
        cells.insert(scope.to_string(), scope_cells);
    }
    Corpus { cells }
}

fn run_surface_requests(root: &Path, scope: &str) -> Vec<String> {
    let options = server_options(root);
    let mut responses = Vec::new();

    // GET cell for every id in this scope.
    for id in 1000..=1000 + (SCOPES.len() * 3) as u64 {
        let target = format!("/v1/cell?cell_id={}", id);
        let response = handle_http_with_options(root, &http_request("GET", &target, ""), &options);
        responses.push(response);
    }

    let aql = format!(
        "RETRIEVE CONTEXT FOR TASK \"content\" IN BRAIN {brain} LIMIT 10 CANDIDATES;",
        brain = "default"
    );
    responses.push(handle_http_with_options(
        root,
        &http_request("POST", &format!("/v1/aql?scope={}", scope), &aql),
        &options,
    ));

    responses.push(handle_http_with_options(
        root,
        &http_request(
            "POST",
            &format!("/v1/aql?scope={}", scope),
            &format!("EXPLAIN {}", aql),
        ),
        &options,
    ));

    // Negative: explicit WHERE on another scope must still be filtered.
    responses.push(handle_http_with_options(
        root,
        &http_request(
            "POST",
            &format!("/v1/aql?scope={}", scope),
            "RETRIEVE CONTEXT FOR TASK \"content\" IN BRAIN default WHERE scope=beta LIMIT 10 CANDIDATES;",

        ),
        &options,
    ));

    responses.push(handle_http_with_options(
        root,
        &http_request(
            "POST",
            &format!("/v1/search?scope={}&q=content&limit=10", scope),
            "",
        ),
        &options,
    ));

    responses.push(handle_http_with_options(
        root,
        &http_request(
            "POST",
            &format!("/v1/search/explain?scope={}&q=content&limit=10", scope),
            "",
        ),
        &options,
    ));

    responses.push(handle_http_with_options(
        root,
        &http_request(
            "POST",
            &format!("/v1/context?scope={}&format=json", scope),
            &aql,
        ),
        &options,
    ));

    responses.push(handle_http_with_options(
        root,
        &http_request(
            "POST",
            &format!("/v1/context/trace?scope={}&format=json", scope),
            &format!(
                "{{\"retrieve_aql\":\"{}\",\"verify_aql\":\"VERIFY FACT \\\"content\\\" IN BRAIN default;\"}}",
                aql.replace('"', "\\\"")
            ),
        ),
        &options,
    ));

    responses.push(handle_http_with_options(
        root,
        &http_request(
            "POST",
            &format!("/v1/verify?scope={}&format=json", scope),
            "VERIFY FACT \"content\" IN BRAIN default;",
        ),
        &options,
    ));
    responses.push(handle_http_with_options(
        root,
        &http_request("GET", &format!("/v1/conflicts?scope={}", scope), ""),
        &options,
    ));

    responses
}

fn assert_no_leak(responses: &[String], forbidden: &[String]) {
    for response in responses {
        let body = response_body(response);
        for secret in forbidden {
            assert!(
                !body.contains(secret),
                "permission leak: secret {secret:?} found in response body:\n{body}"
            );
        }
    }
}

#[test]
fn agent_view_property_no_cross_scope_leaks_memtable() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let mut rng = Rng::new(0x_a9e1_482b_73f5_6c11);
    let corpus = build_corpus(&mut db, &mut rng);

    let all_scopes: BTreeSet<ScopeId> = SCOPES.iter().map(|s| scope_id(s)).collect();
    let readable = random_subset(&mut rng, &all_scopes);
    let view = build_view(readable.clone());
    db.save_agent_view(&view).unwrap();
    drop(db);

    for scope in SCOPES {
        let responses = run_surface_requests(dir.path(), scope);
        let readable_set = readable.clone();
        for other in SCOPES {
            if !readable_set.contains(&scope_id(other)) {
                let forbidden: Vec<String> = corpus.secrets_for_scope(other).to_vec();
                assert_no_leak(&responses, &forbidden);
            }
        }
    }
}

#[test]
fn agent_view_property_no_cross_scope_leaks_after_flush() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let mut rng = Rng::new(0x_b7c3_591d_64e2_8a33);
    let corpus = build_corpus(&mut db, &mut rng);

    let all_scopes: BTreeSet<ScopeId> = SCOPES.iter().map(|s| scope_id(s)).collect();
    let readable = random_subset(&mut rng, &all_scopes);
    let view = build_view(readable.clone());
    db.save_agent_view(&view).unwrap();
    db.checkpoint().unwrap();
    drop(db);

    for scope in SCOPES {
        let responses = run_surface_requests(dir.path(), scope);
        let readable_set = readable.clone();
        for other in SCOPES {
            if !readable_set.contains(&scope_id(other)) {
                let forbidden: Vec<String> = corpus.secrets_for_scope(other).to_vec();
                assert_no_leak(&responses, &forbidden);
            }
        }
    }
}
