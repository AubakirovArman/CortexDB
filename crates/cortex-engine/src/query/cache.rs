use std::collections::{BTreeMap, VecDeque};
use std::hash::{Hash, Hasher};

use cortex_aql::{parse_aql, AgentView, AqlStatement, Binder, BoundPlan};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::options::DEFAULT_AQL_QUERY_CACHE_MAX_ENTRIES;

use super::render::condition_to_string;
use super::EngineAqlIndex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AqlStatementKind {
    Retrieve,
    ExplainRetrieve,
    ExplainAnalyzeRetrieve,
    ExplainOther,
    VerifyFact,
    Remember,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedAqlPlan {
    pub statement_kind: AqlStatementKind,
    pub bound_plan: BoundPlan,
    pub where_expression: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlQueryCacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub catalog_invalidations: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AqlQueryCacheKey {
    query: String,
    view_fingerprint: u64,
    catalog_fingerprint: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct AqlQueryCache {
    entries: BTreeMap<AqlQueryCacheKey, CachedAqlPlan>,
    order: VecDeque<AqlQueryCacheKey>,
    max_entries: usize,
    active_catalog_fingerprint: Option<u64>,
    hits: u64,
    misses: u64,
    evictions: u64,
    catalog_invalidations: u64,
}

impl Default for AqlQueryCache {
    fn default() -> Self {
        Self::new(DEFAULT_AQL_QUERY_CACHE_MAX_ENTRIES)
    }
}

impl AqlQueryCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
            active_catalog_fingerprint: None,
            hits: 0,
            misses: 0,
            evictions: 0,
            catalog_invalidations: 0,
        }
    }

    pub fn key(query: &str, view: &AgentView, catalog_fingerprint: u64) -> AqlQueryCacheKey {
        AqlQueryCacheKey {
            query: query.to_owned(),
            view_fingerprint: agent_view_fingerprint(view),
            catalog_fingerprint,
        }
    }

    pub fn prepare_catalog(&mut self, catalog_fingerprint: u64) {
        match self.active_catalog_fingerprint {
            None => self.active_catalog_fingerprint = Some(catalog_fingerprint),
            Some(active) if active == catalog_fingerprint => {}
            Some(_) => {
                self.entries.clear();
                self.order.clear();
                self.active_catalog_fingerprint = Some(catalog_fingerprint);
                self.catalog_invalidations += 1;
            }
        }
    }

    pub fn get(&mut self, key: &AqlQueryCacheKey) -> Option<CachedAqlPlan> {
        if let Some(plan) = self.entries.get(key) {
            self.hits += 1;
            Some(plan.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, key: AqlQueryCacheKey, plan: CachedAqlPlan) {
        if !self.entries.contains_key(&key) {
            self.order.push_back(key.clone());
        }
        self.entries.insert(key, plan);
        while self.entries.len() > self.max_entries {
            if let Some(oldest) = self.order.pop_front() {
                if self.entries.remove(&oldest).is_some() {
                    self.evictions += 1;
                }
            } else {
                break;
            }
        }
    }

    pub fn stats(&self) -> AqlQueryCacheStats {
        AqlQueryCacheStats {
            entries: self.entries.len(),
            max_entries: self.max_entries,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            catalog_invalidations: self.catalog_invalidations,
        }
    }
}

impl CachedAqlPlan {
    pub fn from_statement(statement: &AqlStatement<'_>, bound_plan: BoundPlan) -> Self {
        match statement {
            AqlStatement::RetrieveContext(raw) => Self {
                statement_kind: AqlStatementKind::Retrieve,
                bound_plan,
                where_expression: raw
                    .where_clause
                    .as_ref()
                    .map(|condition| condition_to_string(&condition.node)),
            },
            AqlStatement::Explain(inner) => match inner.as_ref() {
                AqlStatement::RetrieveContext(raw) => Self {
                    statement_kind: AqlStatementKind::ExplainRetrieve,
                    bound_plan,
                    where_expression: raw
                        .where_clause
                        .as_ref()
                        .map(|condition| condition_to_string(&condition.node)),
                },
                _ => Self {
                    statement_kind: AqlStatementKind::ExplainOther,
                    bound_plan,
                    where_expression: None,
                },
            },
            AqlStatement::ExplainAnalyze(inner) => match inner.as_ref() {
                AqlStatement::RetrieveContext(raw) => Self {
                    statement_kind: AqlStatementKind::ExplainAnalyzeRetrieve,
                    bound_plan,
                    where_expression: raw
                        .where_clause
                        .as_ref()
                        .map(|condition| condition_to_string(&condition.node)),
                },
                _ => Self {
                    statement_kind: AqlStatementKind::ExplainOther,
                    bound_plan,
                    where_expression: None,
                },
            },
            AqlStatement::VerifyFact(_) => Self {
                statement_kind: AqlStatementKind::VerifyFact,
                bound_plan,
                where_expression: None,
            },
            AqlStatement::Remember(_) => Self {
                statement_kind: AqlStatementKind::Remember,
                bound_plan,
                where_expression: None,
            },
        }
    }
}

impl Database {
    pub fn aql_query_cache_stats(&self) -> EngineResult<AqlQueryCacheStats> {
        Ok(self
            .aql_query_cache
            .lock()
            .map_err(|_| cache_lock_error())?
            .stats())
    }

    pub(crate) fn bind_aql_cached(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<(CachedAqlPlan, EngineAqlIndex)> {
        let catalog_fingerprint = self.aql_catalog_fingerprint();
        let key = AqlQueryCache::key(aql, view, catalog_fingerprint);
        if let Some(cached) = {
            let mut cache = self
                .aql_query_cache
                .lock()
                .map_err(|_| cache_lock_error())?;
            cache.prepare_catalog(catalog_fingerprint);
            cache.get(&key)
        } {
            let index = self.try_aql_index_for_bound_plan(view, &cached.bound_plan)?;
            return Ok((cached, index));
        }

        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let catalog = self.aql_statistics_catalog();
        let bound = Binder::new(&catalog, view).bind_statement(&statement)?;
        let cached = CachedAqlPlan::from_statement(&statement, bound);
        let index = self.try_aql_index_for_bound_plan(view, &cached.bound_plan)?;
        self.aql_query_cache
            .lock()
            .map_err(|_| cache_lock_error())?
            .insert(key, cached.clone());
        Ok((cached, index))
    }

    pub(crate) fn bind_verify_fact_cached(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<CachedAqlPlan> {
        let catalog_fingerprint = self.aql_catalog_fingerprint();
        let key = AqlQueryCache::key(aql, view, catalog_fingerprint);
        if let Some(cached) = {
            let mut cache = self
                .aql_query_cache
                .lock()
                .map_err(|_| cache_lock_error())?;
            cache.prepare_catalog(catalog_fingerprint);
            cache.get(&key)
        } {
            return Ok(cached);
        }

        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let catalog = EngineAqlIndex::default();
        let bound = Binder::new(&catalog, view).bind_statement(&statement)?;
        let cached = CachedAqlPlan::from_statement(&statement, bound);
        self.aql_query_cache
            .lock()
            .map_err(|_| cache_lock_error())?
            .insert(key, cached.clone());
        Ok(cached)
    }

    fn aql_catalog_fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.current_seq.0.hash(&mut hasher);
        self.manifest.generation.hash(&mut hasher);
        self.manifest.checkpoint_seq.hash(&mut hasher);
        for segment in &self.manifest.live_segments {
            segment.id.hash(&mut hasher);
            segment.generation.hash(&mut hasher);
            segment.checkpoint_seq.hash(&mut hasher);
            segment.cell_count.hash(&mut hasher);
        }
        hasher.finish()
    }
}

pub(crate) fn cache_lock_error() -> EngineError {
    EngineError::StorageInvariant("AQL query cache lock is poisoned".to_owned())
}

fn agent_view_fingerprint(view: &AgentView) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    view.agent_id.hash(&mut hasher);
    view.label.hash(&mut hasher);
    view.readable_brains.hash(&mut hasher);
    view.readable_scopes.hash(&mut hasher);
    view.writable_scopes.hash(&mut hasher);
    view.allowed_modes.hash(&mut hasher);
    view.allowed_memory_types.hash(&mut hasher);
    view.max_context_budget_tokens.hash(&mut hasher);
    view.default_context_budget_tokens.hash(&mut hasher);
    view.max_candidate_limit.hash(&mut hasher);
    view.default_candidate_limit.hash(&mut hasher);
    view.min_required_confidence_q16.hash(&mut hasher);
    view.max_ttl_seconds.hash(&mut hasher);
    view.allow_remember.hash(&mut hasher);
    view.allow_verify_fact.hash(&mut hasher);
    view.allow_audit_mode.hash(&mut hasher);
    view.require_citations_by_default.hash(&mut hasher);
    view.private_scope.hash(&mut hasher);
    hasher.finish()
}
