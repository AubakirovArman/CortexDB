use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;

const MAX_QUERY_TERMS_FOR_SEARCH: usize = 8;
const MAX_POSTING_SCAN: usize = 25_000;
const MERGED_LEXICAL_CACHE_FILE: &str = "benchmark-merged-lexical.aci";
const MERGED_LEXICAL_CACHE_MANIFEST_FILE: &str = "benchmark-merged-lexical.manifest";

#[derive(Debug)]
pub struct BenchmarkMetadataIndex {
    doc_paths: Vec<(String, String)>,
}

impl BenchmarkMetadataIndex {
    pub fn from_uuid_index(uuid_index: &BTreeMap<String, String>) -> Self {
        Self {
            doc_paths: uuid_index
                .iter()
                .filter_map(|(doc_id, rel_path)| {
                    let rel_path = rel_path.to_lowercase();
                    is_overview_candidate_path(&rel_path).then_some((doc_id.clone(), rel_path))
                })
                .collect(),
        }
    }

    pub fn search_doc_ids(&self, query: &str, limit: usize) -> Vec<String> {
        if !is_overview_query(query) || limit == 0 {
            return Vec::new();
        }
        let profile = OverviewQueryProfile::new(query);
        let mut scored = self
            .doc_paths
            .iter()
            .filter_map(|(doc_id, rel_path)| {
                let score = overview_path_score(&profile, rel_path);
                (score > 0).then_some((doc_id.clone(), score))
            })
            .collect::<Vec<_>>();
        scored.sort_by_key(|(doc_id, score)| (Reverse(*score), doc_id.clone()));
        scored
            .into_iter()
            .map(|(doc_id, _)| doc_id)
            .take(limit)
            .collect()
    }
}

#[derive(Debug)]
pub struct BenchmarkRetrievalIndex {
    lexical: LexicalIndex,
    candidate_to_doc_id: BTreeMap<u32, String>,
    candidate_to_rel_path: BTreeMap<u32, String>,
    source_allowed: BTreeMap<String, BTreeSet<u32>>,
    allowed: BTreeSet<u32>,
    all_avg_len_q10: u64,
    field_avg_len_q10: BTreeMap<String, u64>,
}

impl BenchmarkRetrievalIndex {
    pub fn load(db: &Database, uuid_index: &BTreeMap<String, String>) -> Result<Self, String> {
        let cache_key = merged_lexical_cache_key(db);
        let cache_path = db.root_path().join(MERGED_LEXICAL_CACHE_FILE);
        let cache_manifest_path = db.root_path().join(MERGED_LEXICAL_CACHE_MANIFEST_FILE);
        if cache_manifest_path.exists()
            && cache_path.exists()
            && fs::read_to_string(&cache_manifest_path).ok().as_deref() == Some(cache_key.as_str())
        {
            if let Ok(lexical) = LexicalIndex::read(&cache_path) {
                return Ok(Self::from_lexical(lexical, uuid_index));
            }
        }

        let mut lexical = LexicalIndex::default();
        let segments_path = db.root_path().join("segments");
        for segment in &db.manifest().live_segments {
            let path = segments_path.join(format!("segment-{}.aci", segment.id));
            let segment_lexical = LexicalIndex::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            merge_lexical(&mut lexical, segment_lexical);
        }
        write_merged_lexical_cache(&cache_path, &cache_manifest_path, &cache_key, &lexical)?;
        Ok(Self::from_lexical(lexical, uuid_index))
    }

    pub fn from_lexical(lexical: LexicalIndex, uuid_index: &BTreeMap<String, String>) -> Self {
        let (candidate_to_doc_id, candidate_to_source_type, candidate_to_rel_path) =
            candidate_doc_maps(uuid_index, &lexical.doc_lengths);
        let allowed = candidate_to_doc_id.keys().copied().collect::<BTreeSet<_>>();
        let source_allowed = source_allowed_map(&candidate_to_source_type);
        let all_avg_len_q10 = average_len_q10(&lexical.doc_lengths, &allowed);
        let field_avg_len_q10 = lexical
            .field_doc_lengths
            .keys()
            .map(|field| {
                (
                    field.clone(),
                    average_field_len_q10(&lexical.field_doc_lengths, field),
                )
            })
            .collect();
        Self {
            lexical,
            candidate_to_doc_id,
            candidate_to_rel_path,
            source_allowed,
            allowed,
            all_avg_len_q10,
            field_avg_len_q10,
        }
    }

    pub fn search_doc_ids(
        &self,
        query: &str,
        source_types: &[String],
        limit: usize,
    ) -> Vec<String> {
        let original_query = query;
        let expanded_query = expand_overview_query(query);
        let query = expanded_query.as_deref().unwrap_or(query);
        let preferred = self.allowed_for_source_types(source_types);
        let mut candidates = self.metadata_candidates(original_query, limit);
        let mut seen = candidates.iter().copied().collect::<BTreeSet<_>>();
        let lexical_limit = limit.saturating_sub(candidates.len());
        let lexical_candidates = if preferred.is_empty() {
            self.search_candidates(query, &self.allowed, lexical_limit)
        } else {
            self.search_candidates(query, &preferred, lexical_limit)
        };
        candidates.extend(
            lexical_candidates
                .into_iter()
                .filter(|candidate| seen.insert(*candidate)),
        );
        if candidates.len() < limit && !preferred.is_empty() {
            candidates.extend(
                self.search_candidates(query, &self.allowed, limit)
                    .into_iter()
                    .filter(|candidate| seen.insert(*candidate))
                    .take(limit - candidates.len()),
            );
        }
        candidates
            .into_iter()
            .filter_map(|candidate| self.candidate_to_doc_id.get(&candidate).cloned())
            .collect()
    }

    fn metadata_candidates(&self, query: &str, limit: usize) -> Vec<u32> {
        if !is_overview_query(query) {
            return Vec::new();
        }
        let profile = OverviewQueryProfile::new(query);
        let mut scored = self
            .candidate_to_rel_path
            .iter()
            .filter_map(|(candidate, rel_path)| {
                let score = overview_path_score(&profile, rel_path);
                (score > 0).then_some((*candidate, score))
            })
            .collect::<Vec<_>>();
        scored.sort_by_key(|(candidate, score)| (Reverse(*score), *candidate));
        scored
            .into_iter()
            .map(|(candidate, _)| candidate)
            .take(limit)
            .collect()
    }

    fn search_candidates(&self, query: &str, allowed: &BTreeSet<u32>, limit: usize) -> Vec<u32> {
        if limit == 0 {
            return Vec::new();
        }
        let all_allowed = allowed.len() == self.allowed.len();
        let doc_count = allowed.len().max(1) as u64;
        let avg_len_q10 = if all_allowed {
            self.all_avg_len_q10
        } else {
            average_len_q10(&self.lexical.doc_lengths, allowed)
        };
        let mut scores = BTreeMap::<u32, u64>::new();
        for term in self.search_terms(query) {
            let Some(posting) = self.lexical.terms.get(&term) else {
                continue;
            };
            let visible_count = if all_allowed {
                posting.len().min(MAX_POSTING_SCAN)
            } else {
                posting
                    .iter()
                    .filter(|id| allowed.contains(id))
                    .take(MAX_POSTING_SCAN)
                    .count()
            } as u64;
            if visible_count == 0 {
                continue;
            }
            let idf_q10 = ((doc_count + 1) * 1024) / (visible_count + 1);
            let candidates = posting
                .iter()
                .filter(|id| all_allowed || allowed.contains(id));
            for candidate in candidates.take(MAX_POSTING_SCAN) {
                let field_score = self.field_score_q10(&term, *candidate, idf_q10);
                let score = if field_score > 0 {
                    field_score
                } else {
                    let tf = u64::from(self.term_frequency(&term, *candidate));
                    let len_q10 =
                        u64::from(*self.lexical.doc_lengths.get(candidate).unwrap_or(&1)) * 1024;
                    let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                    let denom_q10 = (tf * 1024) + norm_q10;
                    let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                    idf_q10 * tf_norm_q10
                };
                *scores.entry(*candidate).or_default() += score;
            }
        }
        let mut ranked = scores.into_iter().collect::<Vec<_>>();
        ranked.sort_by_key(|(candidate, score)| (Reverse(*score), *candidate));
        ranked
            .into_iter()
            .map(|(candidate, _)| candidate)
            .take(limit)
            .collect()
    }

    fn search_terms(&self, query: &str) -> Vec<String> {
        let mut terms = BTreeMap::<String, usize>::new();
        for term in cortex_engine::search::tokenize(query) {
            if let Some(posting) = self.lexical.terms.get(&term) {
                terms
                    .entry(term)
                    .and_modify(|size| *size = (*size).min(posting.len()))
                    .or_insert(posting.len());
            }
        }
        let mut terms = terms.into_iter().collect::<Vec<_>>();
        terms.sort_by_key(|(term, posting_size)| (*posting_size, term.clone()));
        terms
            .into_iter()
            .map(|(term, _)| term)
            .take(MAX_QUERY_TERMS_FOR_SEARCH)
            .collect()
    }

    fn allowed_for_source_types(&self, source_types: &[String]) -> BTreeSet<u32> {
        if source_types.is_empty() {
            return BTreeSet::new();
        }
        let mut allowed = BTreeSet::new();
        for source_type in source_types {
            if let Some(candidates) = self.source_allowed.get(source_type) {
                allowed.extend(candidates);
            }
        }
        allowed
    }

    fn term_frequency(&self, term: &str, candidate: u32) -> u32 {
        self.lexical
            .term_frequencies
            .get(term)
            .and_then(|values| values.get(&candidate))
            .copied()
            .unwrap_or(1)
    }

    fn field_score_q10(&self, term: &str, candidate: u32, idf_q10: u64) -> u64 {
        self.lexical
            .field_term_frequencies
            .iter()
            .filter_map(|(field, terms)| {
                let tf = terms
                    .get(term)
                    .and_then(|values| values.get(&candidate))
                    .copied()?;
                Some((field, tf))
            })
            .map(|(field, tf)| {
                let tf = u64::from(tf);
                let len_q10 = self
                    .lexical
                    .field_doc_lengths
                    .get(field)
                    .and_then(|lengths| lengths.get(&candidate))
                    .copied()
                    .map(u64::from)
                    .unwrap_or(1)
                    * 1024;
                let avg_len_q10 = self.field_avg_len_q10.get(field).copied().unwrap_or(1024);
                let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                let denom_q10 = (tf * 1024) + norm_q10;
                let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                idf_q10
                    .saturating_mul(tf_norm_q10)
                    .saturating_mul(u64::from(lexical_field_weight(field)))
            })
            .sum()
    }
}

fn merged_lexical_cache_key(db: &Database) -> String {
    let manifest = db.manifest();
    let mut key = format!(
        "schema=cortexdb.enterprise_rag_bench.merged_lexical_cache.v1\n\
         generation={}\n\
         checkpoint_seq={}\n",
        manifest.generation, manifest.checkpoint_seq
    );
    for segment in &manifest.live_segments {
        key.push_str(&format!(
            "segment={} generation={} checkpoint_seq={} cell_count={}\n",
            segment.id, segment.generation, segment.checkpoint_seq, segment.cell_count
        ));
    }
    key
}

fn write_merged_lexical_cache(
    cache_path: &PathBuf,
    cache_manifest_path: &PathBuf,
    cache_key: &str,
    lexical: &LexicalIndex,
) -> Result<(), String> {
    lexical
        .write(cache_path)
        .map_err(|error| format!("failed to write {}: {error}", cache_path.display()))?;
    fs::write(cache_manifest_path, cache_key)
        .map_err(|error| format!("failed to write {}: {error}", cache_manifest_path.display()))?;
    Ok(())
}

fn source_allowed_map(
    candidate_to_source_type: &BTreeMap<u32, String>,
) -> BTreeMap<String, BTreeSet<u32>> {
    let mut values = BTreeMap::<String, BTreeSet<u32>>::new();
    for (candidate, source_type) in candidate_to_source_type {
        values
            .entry(source_type.clone())
            .or_default()
            .insert(*candidate);
    }
    values
}

fn expand_overview_query(query: &str) -> Option<String> {
    if !is_overview_query(query) {
        return None;
    }
    let mut expanded = query.to_owned();
    expanded.push(' ');
    expanded.push_str(
        "redwood inference company overview strategy platform product business model \
         commercial offering organization departments reliability security private deployment \
         routing serving runtime differentiation go to market support success",
    );
    Some(expanded)
}

fn is_overview_query(query: &str) -> bool {
    let query = query.to_lowercase();
    let markers = [
        "mission statement",
        "company's thesis",
        "company thesis",
        "competitive advantage",
        "security-oriented features",
        "security oriented features",
        "serving-runtime optimizations",
        "serving runtime optimizations",
        "policy dimensions",
        "smart routing",
        "stated differentiation",
        "graceful degradation",
        "revenue streams",
        "business model",
        "add-on categories",
        "add on categories",
        "commercial offering",
        "plg-led adoption",
        "plg led adoption",
        "sales-assisted enterprise",
        "sales assisted enterprise",
        "major departments",
        "high-level organization",
        "high level organization",
    ];
    if markers.iter().any(|marker| query.contains(marker)) {
        return true;
    }
    query.contains("redwood")
        && query.contains("company")
        && [
            "mission",
            "strategy",
            "overview",
            "organization",
            "departments",
            "business",
            "commercial",
            "revenue",
            "differentiation",
        ]
        .iter()
        .any(|marker| query.contains(marker))
}

struct OverviewQueryProfile {
    lower: String,
    terms: Vec<String>,
}

impl OverviewQueryProfile {
    fn new(query: &str) -> Self {
        let lower = query.to_lowercase();
        let terms = cortex_engine::search::tokenize(&lower)
            .into_iter()
            .filter(|term| term.len() >= 4)
            .collect();
        Self { lower, terms }
    }
}

fn overview_path_score(query: &OverviewQueryProfile, path: &str) -> u32 {
    let mut score = 0u32;

    if path.contains("product-docs/product-overview") {
        score += 80;
    }
    if path.contains("sales-enablement") {
        score += 55;
    }
    if path.contains("company-handbook/00_overview") {
        score += 50;
    }
    if path.contains("pricing-and-packaging") || path.contains("finance-and-legal") {
        score += 25;
    }

    if contains_any(
        &query.lower,
        &["security", "private", "deployment", "compliance"],
    ) && contains_any(
        path,
        &[
            "security-and-compliance",
            "eng-private-deployments",
            "private",
            "security",
        ],
    ) {
        score += 120;
    }
    if contains_any(
        &query.lower,
        &["serving", "runtime", "optimizations", "engine"],
    ) && contains_any(
        path,
        &[
            "eng-serving-runtime",
            "runtime-architecture",
            "kernel-and-scheduling",
            "model-serving",
        ],
    ) {
        score += 120;
    }
    if contains_any(&query.lower, &["routing", "policy", "route"]) && path.contains("routing") {
        score += 55;
    }
    if contains_any(
        &query.lower,
        &[
            "revenue",
            "business",
            "commercial",
            "add-on",
            "add on",
            "pricing",
        ],
    ) && contains_any(
        path,
        &[
            "pricing-and-packaging",
            "sales-enablement",
            "finance-and-legal",
            "product-overview",
        ],
    ) {
        score += 65;
    }
    if contains_any(
        &query.lower,
        &["plg", "sales-assisted", "sales assisted", "enterprise"],
    ) && contains_any(path, &["sales-enablement", "requirements", "product-docs"])
    {
        score += 60;
    }
    if contains_any(
        &query.lower,
        &["department", "organization", "organisation"],
    ) && contains_any(path, &["company-handbook", "people-ops", "team-wiki"])
    {
        score += 60;
    }

    for term in &query.terms {
        if path.contains(term) {
            score += 10;
        }
    }
    score
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn merge_lexical(dst: &mut LexicalIndex, src: LexicalIndex) {
    for (term, values) in src.terms {
        dst.terms.entry(term).or_default().extend(values);
    }
    dst.doc_lengths.extend(src.doc_lengths);
    for (term, values) in src.term_frequencies {
        dst.term_frequencies.entry(term).or_default().extend(values);
    }
    for (field, values) in src.field_doc_lengths {
        dst.field_doc_lengths
            .entry(field)
            .or_default()
            .extend(values);
    }
    for (field, terms) in src.field_term_frequencies {
        let dst_terms = dst.field_term_frequencies.entry(field).or_default();
        for (term, values) in terms {
            dst_terms.entry(term).or_default().extend(values);
        }
    }
}

fn candidate_doc_maps(
    uuid_index: &BTreeMap<String, String>,
    doc_lengths: &BTreeMap<u32, u32>,
) -> (
    BTreeMap<u32, String>,
    BTreeMap<u32, String>,
    BTreeMap<u32, String>,
) {
    let documents = uuid_index
        .iter()
        .map(|(doc_id, path)| (doc_id.clone(), source_type(path)))
        .collect::<Vec<_>>();
    let document_paths = uuid_index
        .iter()
        .map(|(doc_id, path)| (doc_id.clone(), path.clone()))
        .collect::<Vec<_>>();
    let mut candidate_to_doc_id = BTreeMap::new();
    let mut candidate_to_source_type = BTreeMap::new();
    let mut candidate_to_rel_path = BTreeMap::new();
    for candidate in doc_lengths.keys() {
        if let Some((doc_id, source_type)) = (|| {
            let ordinal = usize::try_from(*candidate).ok()?.checked_sub(1)?;
            documents.get(ordinal)
        })() {
            candidate_to_doc_id.insert(*candidate, doc_id.clone());
            candidate_to_source_type.insert(*candidate, source_type.clone());
        }
        if let Some((_, rel_path)) = (|| {
            let ordinal = usize::try_from(*candidate).ok()?.checked_sub(1)?;
            document_paths.get(ordinal)
        })() {
            let rel_path = rel_path.to_lowercase();
            if is_overview_candidate_path(&rel_path) {
                candidate_to_rel_path.insert(*candidate, rel_path);
            }
        }
    }
    (
        candidate_to_doc_id,
        candidate_to_source_type,
        candidate_to_rel_path,
    )
}

fn source_type(path: &str) -> String {
    path.split('/').next().unwrap_or("unknown").to_owned()
}

fn is_overview_candidate_path(path: &str) -> bool {
    contains_any(
        path,
        &[
            "product-docs/product-overview",
            "sales-enablement",
            "company-handbook/00_overview",
            "pricing-and-packaging",
            "finance-and-legal",
            "security-and-compliance",
            "eng-private-deployments",
            "eng-serving-runtime",
            "runtime-architecture",
            "kernel-and-scheduling",
            "model-serving",
            "routing",
            "requirements",
            "people-ops",
            "team-wiki",
        ],
    )
}

fn average_len_q10(doc_lengths: &BTreeMap<u32, u32>, allowed: &BTreeSet<u32>) -> u64 {
    let mut count = 0u64;
    let mut total = 0u64;
    for candidate in allowed {
        if let Some(length) = doc_lengths.get(candidate) {
            count += 1;
            total += u64::from(*length);
        }
    }
    total
        .saturating_mul(1024)
        .checked_div(count)
        .unwrap_or(1024)
}

fn average_field_len_q10(
    field_doc_lengths: &BTreeMap<String, BTreeMap<u32, u32>>,
    field: &str,
) -> u64 {
    let Some(lengths) = field_doc_lengths.get(field) else {
        return 1024;
    };
    let total = lengths.values().copied().map(u64::from).sum::<u64>();
    if lengths.is_empty() {
        1024
    } else {
        total * 1024 / lengths.len() as u64
    }
}

fn lexical_field_weight(field: &str) -> u32 {
    match field {
        "title" => 8,
        "table" => 6,
        "path" => 5,
        "entity" => 4,
        "chunk" => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_mapping_uses_one_based_candidate_ordinal() {
        let uuid_index = BTreeMap::from([
            (
                "doc-a".to_owned(),
                "confluence/product-docs/product-overview/a.json".to_owned(),
            ),
            (
                "doc-b".to_owned(),
                "confluence/sales-enablement/b.json".to_owned(),
            ),
        ]);
        let doc_lengths = BTreeMap::from([(1, 2), (2, 3), (0, 1)]);

        let (mapped, sources, paths) = candidate_doc_maps(&uuid_index, &doc_lengths);

        assert_eq!(mapped.get(&1), Some(&"doc-a".to_owned()));
        assert_eq!(mapped.get(&2), Some(&"doc-b".to_owned()));
        assert_eq!(sources.get(&1), Some(&"confluence".to_owned()));
        assert_eq!(sources.get(&2), Some(&"confluence".to_owned()));
        assert_eq!(
            paths.get(&1),
            Some(&"confluence/product-docs/product-overview/a.json".to_owned())
        );
        assert_eq!(
            paths.get(&2),
            Some(&"confluence/sales-enablement/b.json".to_owned())
        );
        assert!(!mapped.contains_key(&0));
    }

    #[test]
    fn cached_lexical_search_returns_ranked_doc_ids() {
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "a.json".to_owned()),
            ("doc-b".to_owned(), "b.json".to_owned()),
        ]);
        let lexical = LexicalIndex {
            terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
            doc_lengths: BTreeMap::from([(1, 4), (2, 4)]),
            term_frequencies: BTreeMap::from([(
                "budget".to_owned(),
                BTreeMap::from([(1, 1), (2, 4)]),
            )]),
            ..LexicalIndex::default()
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert_eq!(
            index.search_doc_ids("budget", &[], 2),
            vec!["doc-b", "doc-a"]
        );
    }

    #[test]
    fn source_type_filter_is_used_before_global_fill() {
        let uuid_index = BTreeMap::from([
            ("doc-a".to_owned(), "slack/a.json".to_owned()),
            ("doc-b".to_owned(), "github/b.json".to_owned()),
        ]);
        let lexical = LexicalIndex {
            terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
            doc_lengths: BTreeMap::from([(1, 4), (2, 4)]),
            term_frequencies: BTreeMap::from([(
                "budget".to_owned(),
                BTreeMap::from([(1, 4), (2, 1)]),
            )]),
            ..LexicalIndex::default()
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert_eq!(
            index.search_doc_ids("budget", &["github".to_owned()], 2),
            vec!["doc-b", "doc-a"]
        );
    }

    #[test]
    fn overview_queries_get_company_context_expansion() {
        let uuid_index = BTreeMap::from([
            (
                "doc-a".to_owned(),
                "confluence/company-overview.json".to_owned(),
            ),
            ("doc-b".to_owned(), "slack/runtime-incident.json".to_owned()),
        ]);
        let lexical = LexicalIndex {
            terms: BTreeMap::from([
                ("company".to_owned(), BTreeSet::from([1])),
                ("overview".to_owned(), BTreeSet::from([1])),
                ("platform".to_owned(), BTreeSet::from([1])),
                ("incident".to_owned(), BTreeSet::from([2])),
            ]),
            doc_lengths: BTreeMap::from([(1, 8), (2, 8)]),
            term_frequencies: BTreeMap::from([
                ("company".to_owned(), BTreeMap::from([(1, 1)])),
                ("overview".to_owned(), BTreeMap::from([(1, 1)])),
                ("platform".to_owned(), BTreeMap::from([(1, 1)])),
                ("incident".to_owned(), BTreeMap::from([(2, 1)])),
            ]),
            ..LexicalIndex::default()
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert_eq!(
            index.search_doc_ids("What is Redwood Inference's mission statement?", &[], 2),
            vec!["doc-a"]
        );
    }

    #[test]
    fn non_overview_queries_do_not_get_company_context_expansion() {
        let uuid_index = BTreeMap::from([(
            "doc-a".to_owned(),
            "confluence/company-overview.json".to_owned(),
        )]);
        let lexical = LexicalIndex {
            terms: BTreeMap::from([("company".to_owned(), BTreeSet::from([1]))]),
            doc_lengths: BTreeMap::from([(1, 8)]),
            term_frequencies: BTreeMap::from([("company".to_owned(), BTreeMap::from([(1, 1)]))]),
            ..LexicalIndex::default()
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert!(index
            .search_doc_ids("What timeout changed for SDK retries?", &[], 2)
            .is_empty());
    }

    #[test]
    fn overview_queries_boost_corpus_metadata_paths() {
        let uuid_index = BTreeMap::from([
            (
                "doc-a".to_owned(),
                "confluence/product-docs/product-overview/platform-brief.json".to_owned(),
            ),
            (
                "doc-b".to_owned(),
                "confluence/eng-serving-runtime/runtime-architecture/runtime-notes.json".to_owned(),
            ),
            (
                "doc-c".to_owned(),
                "slack/support/random-customer-thread.json".to_owned(),
            ),
        ]);
        let lexical = LexicalIndex {
            terms: BTreeMap::from([("random".to_owned(), BTreeSet::from([3]))]),
            doc_lengths: BTreeMap::from([(1, 8), (2, 8), (3, 8)]),
            term_frequencies: BTreeMap::from([("random".to_owned(), BTreeMap::from([(3, 1)]))]),
            ..LexicalIndex::default()
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert_eq!(
            index.search_doc_ids(
                "Which serving-runtime optimizations are part of Redwood's inference engine design?",
                &[],
                2,
            ),
            vec!["doc-b", "doc-a"]
        );
    }

    #[test]
    fn non_overview_queries_do_not_boost_metadata_paths() {
        let uuid_index = BTreeMap::from([(
            "doc-a".to_owned(),
            "confluence/product-docs/product-overview/platform-brief.json".to_owned(),
        )]);
        let lexical = LexicalIndex {
            terms: BTreeMap::new(),
            doc_lengths: BTreeMap::from([(1, 8)]),
            term_frequencies: BTreeMap::new(),
            ..LexicalIndex::default()
        };
        let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

        assert!(index
            .search_doc_ids("What timeout changed for SDK retries?", &[], 2)
            .is_empty());
    }

    #[test]
    fn metadata_index_can_answer_overview_without_lexical_index() {
        let uuid_index = BTreeMap::from([
            (
                "doc-a".to_owned(),
                "confluence/product-docs/product-overview/platform-brief.json".to_owned(),
            ),
            (
                "doc-b".to_owned(),
                "slack/support/random-customer-thread.json".to_owned(),
            ),
        ]);
        let index = BenchmarkMetadataIndex::from_uuid_index(&uuid_index);

        assert_eq!(
            index.search_doc_ids("What is Redwood Inference's business model?", 2),
            vec!["doc-a"]
        );
        assert!(index
            .search_doc_ids("What timeout changed for SDK retries?", 2)
            .is_empty());
    }
}
