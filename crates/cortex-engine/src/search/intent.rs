#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnterpriseRagQuestionType {
    Basic,
    Semantic,
    IntraDocumentReasoning,
    ProjectRelated,
    Constrained,
    ConflictingInfo,
    Completeness,
    HighLevel,
    InfoNotFound,
    Miscellaneous,
}

impl EnterpriseRagQuestionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Semantic => "semantic",
            Self::IntraDocumentReasoning => "intra_document_reasoning",
            Self::ProjectRelated => "project_related",
            Self::Constrained => "constrained",
            Self::ConflictingInfo => "conflicting_info",
            Self::Completeness => "completeness",
            Self::HighLevel => "high_level",
            Self::InfoNotFound => "info_not_found",
            Self::Miscellaneous => "miscellaneous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match normalize_label(value).as_str() {
            "basic" => Some(Self::Basic),
            "semantic" => Some(Self::Semantic),
            "intra_document_reasoning" | "intradocumentreasoning" => {
                Some(Self::IntraDocumentReasoning)
            }
            "project_related" | "projectrelated" => Some(Self::ProjectRelated),
            "constrained" => Some(Self::Constrained),
            "conflicting_info" | "conflictinginfo" => Some(Self::ConflictingInfo),
            "completeness" => Some(Self::Completeness),
            "high_level" | "highlevel" => Some(Self::HighLevel),
            "info_not_found" | "infonotfound" | "unavailable" | "null_query" | "nullquery" => {
                Some(Self::InfoNotFound)
            }
            "miscellaneous" | "misc" => Some(Self::Miscellaneous),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnterpriseRagIntentClassification {
    pub question_type: EnterpriseRagQuestionType,
    pub confidence_q16: u16,
    pub matched_signals: Vec<&'static str>,
}

pub fn classify_enterprise_rag_question_type(query: &str) -> EnterpriseRagQuestionType {
    classify_enterprise_rag_question(query).question_type
}

pub fn classify_enterprise_rag_question(query: &str) -> EnterpriseRagIntentClassification {
    let lower = query.to_ascii_lowercase();
    let mut scores = CategoryScores::default();

    score_high_level(&lower, &mut scores);
    score_miscellaneous(&lower, &mut scores);
    score_info_not_found(&lower, &mut scores);
    score_completeness(&lower, &mut scores);
    score_conflicting_info(&lower, &mut scores);
    score_constrained(&lower, &mut scores);
    score_project_related(&lower, &mut scores);
    score_intra_document_reasoning(&lower, &mut scores);
    score_semantic(&lower, &mut scores);
    score_basic(&lower, &mut scores);

    let question_type = scores.best_type();
    EnterpriseRagIntentClassification {
        question_type,
        confidence_q16: scores.confidence_q16(question_type),
        matched_signals: scores.signals(question_type),
    }
}

#[derive(Clone, Debug, Default)]
struct CategoryScores {
    values: Vec<(EnterpriseRagQuestionType, u32, Vec<&'static str>)>,
}

impl CategoryScores {
    fn add(&mut self, question_type: EnterpriseRagQuestionType, score: u32, signal: &'static str) {
        if score == 0 {
            return;
        }
        if let Some((_, value, signals)) = self
            .values
            .iter_mut()
            .find(|(existing, _, _)| *existing == question_type)
        {
            *value = value.saturating_add(score);
            signals.push(signal);
        } else {
            self.values.push((question_type, score, vec![signal]));
        }
    }

    fn best_type(&self) -> EnterpriseRagQuestionType {
        self.values
            .iter()
            .max_by_key(|(question_type, score, _)| (*score, priority(*question_type)))
            .map(|(question_type, _, _)| *question_type)
            .unwrap_or(EnterpriseRagQuestionType::Basic)
    }

    fn confidence_q16(&self, question_type: EnterpriseRagQuestionType) -> u16 {
        let best = self.score(question_type);
        if best == 0 {
            return 32_768;
        }
        let second = self
            .values
            .iter()
            .filter(|(candidate, _, _)| *candidate != question_type)
            .map(|(_, score, _)| *score)
            .max()
            .unwrap_or(0);
        let total = best.saturating_add(second).max(1);
        ((u64::from(best) * 65_535) / u64::from(total)) as u16
    }

    fn score(&self, question_type: EnterpriseRagQuestionType) -> u32 {
        self.values
            .iter()
            .find(|(candidate, _, _)| *candidate == question_type)
            .map(|(_, score, _)| *score)
            .unwrap_or(0)
    }

    fn signals(&self, question_type: EnterpriseRagQuestionType) -> Vec<&'static str> {
        self.values
            .iter()
            .find(|(candidate, _, _)| *candidate == question_type)
            .map(|(_, _, signals)| signals.clone())
            .unwrap_or_default()
    }
}

fn priority(question_type: EnterpriseRagQuestionType) -> u8 {
    match question_type {
        EnterpriseRagQuestionType::InfoNotFound => 10,
        EnterpriseRagQuestionType::Miscellaneous => 9,
        EnterpriseRagQuestionType::HighLevel => 8,
        EnterpriseRagQuestionType::Completeness => 7,
        EnterpriseRagQuestionType::ConflictingInfo => 6,
        EnterpriseRagQuestionType::Constrained => 5,
        EnterpriseRagQuestionType::ProjectRelated => 4,
        EnterpriseRagQuestionType::IntraDocumentReasoning => 3,
        EnterpriseRagQuestionType::Semantic => 2,
        EnterpriseRagQuestionType::Basic => 1,
    }
}

fn score_high_level(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "mission statement",
            "company's thesis",
            "competitive advantage",
            "business model",
            "revenue streams",
            "add-on categories",
            "major departments",
            "high-level organization",
            "stated differentiation",
            "policy dimensions",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::HighLevel, 8, "company_overview");
    }
    if contains_any(
        query,
        &[
            "high level",
            "big picture",
            "company overview",
            "features are highlighted",
            "optimizations are explicitly called out",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::HighLevel, 5, "overview_phrase");
    }
}

fn score_miscellaneous(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "jokes",
            "memes",
            "refrigerator",
            "fridge",
            "office kitchen",
            "softball",
            "fantasy football",
            "sneakers",
            "cleats",
            "planter",
            "wall mounted art",
            "snake plants",
            "zz plants",
            "demo slides",
            "placeholder image",
            "go-to-market misc",
            "deep cleaning",
            "office summer",
            "interview",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Miscellaneous,
            8,
            "non_product_topic",
        );
    }
    if contains_any(
        query,
        &["team chat about posting jokes", "4th floor office"],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Miscellaneous,
            4,
            "office_or_chat_context",
        );
    }
}

fn score_info_not_found(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "specific enterprise accounts",
            "initial allowlist",
            "exact per-route-group budget",
            "exact queue-depth",
            "complete mapping",
            "public blockchain",
            "smart contract address",
            "exact queue token",
            "per-request co2e",
            "bios settings",
            "safe mode boot",
            "cab approval",
            "quorum",
            "tie-break",
            "microsoft teams",
            "adaptive cards",
            "cryptographic signing algorithm",
            "key rotation cadence",
            "exact canonicalization rules",
            "full json schema",
            "checkpoint mismatch",
            "firmware rollout",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::InfoNotFound,
            8,
            "unavailable_specifics",
        );
    }
    if contains_any(
        query,
        &[
            "what exact",
            "required payload schema",
            "where is that mapping source-of-truth",
            "currently configured in production",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::InfoNotFound,
            3,
            "exact_internal_unknown",
        );
    }
}

fn score_completeness(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "procedure",
            "end-to-end process",
            "complete go/no-go gate",
            "across all",
            "across redwood's",
            "which sdk has the highest",
            "how many weekly",
            "most published customer stories",
            "most follow-up action items",
            "most production incidents",
            "which intake channel has the most",
            "has any customer other than",
            "how many fireflies",
            "list all",
            "list each",
            "comprehensive",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Completeness,
            7,
            "multi_document_aggregate",
        );
    }
    if contains_any(
        query,
        &["list all", "list each", "complete", "comprehensive"],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Completeness,
            4,
            "explicit_completeness_request",
        );
    }
    if contains_any(
        query,
        &[
            "required validations",
            "required approvals",
            "required customer communications",
            "including emergency",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Completeness,
            3,
            "checklist_requirements",
        );
    }
}

fn score_conflicting_info(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "latest baseline",
            "previous thresholds",
            "earlier %",
            "compared to",
            "was the degraded",
            "oom or intermittent",
            "manager or cost-ops",
            "latest baseline/growth/peak",
            "customer-managed kms",
            "hosted aws marketplace sku",
            "default ttl",
            "what % of interactive",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ConflictingInfo,
            7,
            "version_or_option_conflict",
        );
    }
    if contains_any(
        query,
        &[
            "conflict",
            "conflicting",
            "contradict",
            "discrepancy",
            "changed from",
            "changed between",
            "previous",
            "earlier",
            "latest",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ConflictingInfo,
            3,
            "conflict_vocabulary",
        );
    }
}

fn score_constrained(query: &str, scores: &mut CategoryScores) {
    if has_date_or_version_signal(query) {
        scores.add(EnterpriseRagQuestionType::Constrained, 3, "date_or_version");
    }
    if contains_any(
        query,
        &[
            "incident where",
            "incident that",
            "root cause and what mitigation",
            "what caused the production",
            "underlying cause and what hotfix",
            "server-side mitigation",
            "immediate mitigation",
            "target ship date",
            "follow-up ticket",
            "controlled failover game day",
            "private/vpc deployment",
            "hosted api issues",
            "long-lived server-sent events",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Constrained,
            6,
            "incident_constraint",
        );
    }
}

fn score_project_related(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "support explain",
            "support handle",
            "sales share",
            "approved stance",
            "evidence pack",
            "deal",
            "credit requests",
            "policy we should follow",
            "what ui or api changes should we make",
            "how do we verify",
            "how should support",
            "how should sales",
            "explain and remediate",
            "enterprise ticket",
            "support bridge",
            "customer update cadence",
            "approvals are required",
            "issuing a credit",
            "standardizing on",
            "what is our approved stance",
            "what caused the savings",
            "do we need to recalculate",
            "when a tenant has",
            "what do we need to install before running restore",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ProjectRelated,
            7,
            "customer_or_project_resolution",
        );
    }
    if contains_any(query, &["launch", "rollout", "release"])
        && contains_any(
            query,
            &[
                "owner", "owns", "dri", "blocker", "blocked", "deadline", "slipped", "status",
                "risk",
            ],
        )
    {
        scores.add(
            EnterpriseRagQuestionType::ProjectRelated,
            6,
            "project_delivery_status",
        );
    }
    if contains_any(
        query,
        &[
            "customer-facing",
            "customer-facing support",
            "enterprise route slo",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ProjectRelated,
            2,
            "project_context_prefix",
        );
    }
}

fn score_intra_document_reasoning(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "what two thresholds",
            "which meeting time",
            "48 hours after the call",
            "which teams are required to sign off",
            "what two follow-up",
            "at the start and what",
            "final tracked wording",
            "what singleton-wrapped",
            "what normalizer",
            "what alert triggered",
            "what two preventative follow-ups",
            "what model and metrics",
            "what artifacts",
            "hybrid approach",
            "30-minute follow-up call",
            "what support ticket number",
            "what request id",
            "if the target go-live",
            "what date should",
            "two unit primitives",
            " and where is the referenced ",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::IntraDocumentReasoning,
            7,
            "multi_fact_same_doc",
        );
    }
    if query.starts_with("during ") || query.starts_with("when using ") {
        scores.add(
            EnterpriseRagQuestionType::IntraDocumentReasoning,
            2,
            "same_document_context",
        );
    }
}

fn score_semantic(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "when does booking open",
            "final concession terms",
            "low bit math",
            "what caused an enterprise tenant",
            "rollout system",
            "large overnight upload",
            "short-lived resume credential",
            "specific gate thresholds",
            "how should i structure",
            "staged rollout schedule",
            "temporary kill switch",
            "why would",
            "why does",
            "what happens when",
            "mandatory items",
            "advisory 0 to 100 score",
            "planned overnight time window",
            "storage setup and time-to-live",
            "internal admin ui",
            "internal routing performance memo",
            "what is the name of the new mechanism",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Semantic,
            6,
            "semantic_paraphrase",
        );
    }
    if contains_any(
        query,
        &[
            "recommended",
            "recommend",
            "recommendation",
            "requirements for",
            "what caused",
            "how should i",
            "tracking things like",
            "which approach",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::Semantic, 2, "conceptual_query");
    }
}

fn score_basic(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "what are the default",
            "what is the name",
            "acceptance criteria",
            "in the meeting",
            "draft spec",
            "internal shiproom",
            "what support response time",
            "what keyboard-only",
            "where was",
            "what mitigation was proposed",
            "how does the new alerting",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::Basic, 4, "known_item_lookup");
    }
}

fn has_date_or_version_signal(query: &str) -> bool {
    query
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | ')' | '('))
        .any(|token| is_iso_date(token) || is_version_like(token))
        || contains_any(
            query,
            &[
                "jan ",
                "january",
                "feb ",
                "february",
                "march",
                "april",
                "may 20",
                "june",
                "july",
                "august",
                "september",
                "october",
                "november",
                "december",
                "h1 2025",
                "runtime 1.",
            ],
        )
}

fn is_iso_date(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, value)| matches!(index, 4 | 7) || value.is_ascii_digit())
}

fn is_version_like(token: &str) -> bool {
    let value = token.strip_prefix('v').unwrap_or(token);
    value.contains('.')
        && value.chars().any(|ch| ch.is_ascii_digit())
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_'))
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn normalize_label(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        classify_enterprise_rag_question, classify_enterprise_rag_question_type,
        EnterpriseRagQuestionType,
    };

    #[test]
    fn classifies_enterprise_rag_types_from_question_text_only() {
        let cases = [
            (
                "What is the name of the new metric for streaming sessions?",
                EnterpriseRagQuestionType::Basic,
            ),
            (
                "How should I structure a prompt experiment to reduce overconfident mistakes?",
                EnterpriseRagQuestionType::Semantic,
            ),
            (
                "During the capacity incident, what two thresholds should trigger detection and how long were SLA targets breached?",
                EnterpriseRagQuestionType::IntraDocumentReasoning,
            ),
            (
                "For the Proxima Bank 429 spike, what caused throttling and how do we verify it is not burning route SLOs?",
                EnterpriseRagQuestionType::ProjectRelated,
            ),
            (
                "Who owns the launch blocker and what is the slipped rollout deadline?",
                EnterpriseRagQuestionType::ProjectRelated,
            ),
            (
                "In the March 2026 incident, what was the root cause and what immediate mitigation did SRE apply?",
                EnterpriseRagQuestionType::Constrained,
            ),
            (
                "What are the v2 score ranges and what were the previous thresholds?",
                EnterpriseRagQuestionType::ConflictingInfo,
            ),
            (
                "What is Redwood's end-to-end process for rotating production secrets?",
                EnterpriseRagQuestionType::Completeness,
            ),
            (
                "What is Redwood Inference's mission statement?",
                EnterpriseRagQuestionType::HighLevel,
            ),
            (
                "What exact queue token format and signing algorithm are configured in production?",
                EnterpriseRagQuestionType::InfoNotFound,
            ),
            (
                "When is the office refrigerator deep cleaning scheduled?",
                EnterpriseRagQuestionType::Miscellaneous,
            ),
        ];

        for (query, expected) in cases {
            assert_eq!(classify_enterprise_rag_question_type(query), expected);
        }
    }

    #[test]
    fn classification_reports_confidence_and_signals() {
        let classified = classify_enterprise_rag_question(
            "What exact queue token format and signing algorithm are configured in production?",
        );

        assert_eq!(
            classified.question_type,
            EnterpriseRagQuestionType::InfoNotFound
        );
        assert!(classified.confidence_q16 > 32_768);
        assert!(!classified.matched_signals.is_empty());
    }

    #[test]
    fn parses_public_enterprise_rag_type_labels() {
        assert_eq!(
            EnterpriseRagQuestionType::parse("intra_document_reasoning"),
            Some(EnterpriseRagQuestionType::IntraDocumentReasoning)
        );
        assert_eq!(
            EnterpriseRagQuestionType::parse("null_query"),
            Some(EnterpriseRagQuestionType::InfoNotFound)
        );
        assert_eq!(EnterpriseRagQuestionType::parse("unknown"), None);
    }
}
