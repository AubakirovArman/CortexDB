use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::verification::VerificationReportExportFormat;
use cortex_engine::{
    scope_id, ContextPackExportFormat, Database, DatabaseOptions, EngineFeatureFlags,
};

const PUBLIC_SCOPE: &str = "project:investments";
const FORBIDDEN_SCOPE: &str = "agent:private";
const PRIVATE_SECRET: &str = "PRIVATE_SCOPE_SHOULD_NOT_LEAK";
const PRIVATE_SOURCE: &str = "private-source";
const PRIVATE_CITATION: &str = "private-citation";
const PRIVATE_JSON_PATH: &str = "private-json-path";

pub(crate) fn assert_no_forbidden(label: &str, text: &str) {
    for marker in [
        PRIVATE_SECRET,
        FORBIDDEN_SCOPE,
        PRIVATE_SOURCE,
        PRIVATE_CITATION,
        PRIVATE_JSON_PATH,
        "https://private.example",
    ] {
        assert!(
            !text.contains(marker),
            "{label} leaked forbidden marker {marker}: {text}"
        );
    }
}

pub(crate) fn seed_scope_leak_fixture(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ntype=fact\nsource=public-source\nsource_id=public-source\ndocument_id=public-doc\npage=1\njson_path=public.path\ncitation=public-citation\nsource_trust_q16=60000\nvector=100,0\n\nSolar Plant budget is 1.2B KZT. public investment budget evidence.".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ntype=fact\nsource=public-source\nsource_id=public-source\ndocument_id=public-doc-2\npage=2\ncitation=public-citation-2\nmetric=budget\nsource_trust_q16=60000\nvector=90,1\n\nSolar Plant budget is 1.4B KZT. public numeric contradiction evidence.".to_vec(),
    )
    .unwrap();
    for id in 10..=25 {
        db.put_cell(
            CellId(id),
            format!(
                "scope={PUBLIC_SCOPE}\nstatus=ready\nsource=public-vector\nvector={},{}\n\npublic vector budget evidence {id}",
                id,
                25 - id
            )
            .into_bytes(),
        )
        .unwrap();
    }
    db.put_cell(
        CellId(900),
        format!(
            "scope={FORBIDDEN_SCOPE}\nstatus=ready\ntype=fact\nsource={PRIVATE_SOURCE}\nsource_id={PRIVATE_SOURCE}\nsource_url=https://private.example/{PRIVATE_SECRET}\ndocument_id=private-doc\njson_path={PRIVATE_JSON_PATH}\ncitation={PRIVATE_CITATION}\nmetric=budget\nvector=32767,32767\n\n{PRIVATE_SECRET} Solar Plant budget is 9.9B KZT."
        )
        .into_bytes(),
    )
    .unwrap();
    for id in 901..=916 {
        db.put_cell(
            CellId(id),
            format!(
                "scope={FORBIDDEN_SCOPE}\nstatus=ready\nsource={PRIVATE_SOURCE}\nvector={},{}\n\n{PRIVATE_SECRET} private vector evidence {id}",
                id,
                916 - id
            )
            .into_bytes(),
        )
        .unwrap();
    }
}

pub(crate) fn public_agent_views() -> Vec<AgentView> {
    vec![public_agent_view(1), public_agent_view(2)]
}

pub(crate) fn hnsw_options() -> DatabaseOptions {
    DatabaseOptions {
        feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
        ..DatabaseOptions::default()
    }
}

pub(crate) fn verify_query() -> &'static str {
    r#"VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;"#
}

#[derive(Clone, Copy)]
pub(crate) enum QueryShape {
    Broad,
    WhereNarrowed,
    LimitOne,
    ExplicitForbidden,
}

impl QueryShape {
    pub(crate) const ALL: [Self; 4] = [
        Self::Broad,
        Self::WhereNarrowed,
        Self::LimitOne,
        Self::ExplicitForbidden,
    ];

    pub(crate) fn query(self) -> &'static str {
        match self {
            Self::Broad => broad_query(),
            Self::WhereNarrowed => where_narrowed_query(),
            Self::LimitOne => limit_one_query(),
            Self::ExplicitForbidden => forbidden_query(),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Broad => "broad",
            Self::WhereNarrowed => "where_narrowed",
            Self::LimitOne => "limit_one",
            Self::ExplicitForbidden => "explicit_forbidden",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum BudgetMode {
    Normal,
    TightToken,
    AnnBudgetExhausted,
}

impl BudgetMode {
    pub(crate) const ALL: [Self; 3] = [Self::Normal, Self::TightToken, Self::AnnBudgetExhausted];

    pub(crate) fn token_budget(self) -> u32 {
        match self {
            Self::Normal | Self::AnnBudgetExhausted => 1_000,
            Self::TightToken => 24,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::TightToken => "tight_token",
            Self::AnnBudgetExhausted => "ann_budget_exhausted",
        }
    }
}

pub(crate) const CONTEXT_FORMATS: [(ContextPackExportFormat, &str); 3] = [
    (ContextPackExportFormat::Json, "json"),
    (ContextPackExportFormat::Prompt, "prompt"),
    (ContextPackExportFormat::Markdown, "markdown"),
];

pub(crate) const VERIFICATION_FORMATS: [(VerificationReportExportFormat, &str); 2] = [
    (VerificationReportExportFormat::Markdown, "markdown"),
    (VerificationReportExportFormat::Audit, "audit"),
];

fn public_agent_view(agent_id: u64) -> AgentView {
    AgentView {
        agent_id: AgentId(agent_id),
        label: Some(format!("public-agent-{agent_id}")),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(PUBLIC_SCOPE)]),
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
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: true,
        private_scope: None,
    }
}

fn broad_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn where_narrowed_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#
}

fn limit_one_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 1 CANDIDATES;"#
}

fn forbidden_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = agent:private AND status = "ready" LIMIT 10 CANDIDATES;"#
}
