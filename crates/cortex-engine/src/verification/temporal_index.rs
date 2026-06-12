use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::query::{scope_id, CellMetadata};

use super::{parse_temporal_date, TemporalDate};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemporalFactKey {
    pub subject: String,
    pub metric: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalFactRecord {
    pub cell_id: CellId,
    pub key: TemporalFactKey,
    pub value: Option<String>,
    pub as_of: Option<TemporalDate>,
    pub created_unix_seconds: Option<u64>,
    pub superseded_by: Option<CellId>,
    pub explicit_supersedes: Option<String>,
    pub explicit_superseded_by: Option<String>,
    pub latest: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemporalFactIndex {
    pub records: Vec<TemporalFactRecord>,
}

impl Database {
    pub fn temporal_fact_index(&self, view: &AgentView) -> TemporalFactIndex {
        let mut groups = BTreeMap::<TemporalFactKey, Vec<TemporalFactRecord>>::new();
        for version in self.snapshot_versions() {
            let metadata = CellMetadata::from_version(&version);
            if !view.can_read_scope(scope_id(&metadata.scope)) {
                continue;
            }
            let Some(key) = temporal_fact_key(&metadata) else {
                continue;
            };
            groups
                .entry(key.clone())
                .or_default()
                .push(TemporalFactRecord {
                    cell_id: version.cell_id,
                    key,
                    value: metadata_value(&metadata.body_text, &["value"]),
                    as_of: temporal_date_for_record(&metadata),
                    created_unix_seconds: metadata.created_unix_seconds,
                    superseded_by: None,
                    explicit_supersedes: metadata.supersedes,
                    explicit_superseded_by: metadata.superseded_by,
                    latest: false,
                });
        }

        let mut records = Vec::new();
        for (_, mut group) in groups {
            group.sort_by_key(temporal_sort_key);
            let latest_cell_id = group.last().map(|record| record.cell_id);
            for record in &mut group {
                record.latest = Some(record.cell_id) == latest_cell_id;
                if !record.latest {
                    record.superseded_by = latest_cell_id;
                }
            }
            records.extend(group);
        }
        records.sort_by_key(|record| {
            (
                record.key.clone(),
                record.as_of,
                record.created_unix_seconds,
                record.cell_id,
            )
        });
        TemporalFactIndex { records }
    }

    pub fn latest_temporal_facts(&self, view: &AgentView) -> Vec<TemporalFactRecord> {
        self.temporal_fact_index(view)
            .records
            .into_iter()
            .filter(|record| record.latest)
            .collect()
    }
}

fn temporal_fact_key(metadata: &CellMetadata) -> Option<TemporalFactKey> {
    let subject = metadata_value(&metadata.body_text, &["subject", "project", "entity"])
        .or_else(|| metadata.project.clone())
        .or_else(|| metadata.entity.clone())?;
    let metric = metadata_value(&metadata.body_text, &["metric"])?;
    Some(TemporalFactKey {
        subject: subject.trim().to_owned(),
        metric: metric.trim().to_owned(),
    })
    .filter(|key| !key.subject.is_empty() && !key.metric.is_empty())
}

fn temporal_date_for_record(metadata: &CellMetadata) -> Option<TemporalDate> {
    metadata
        .as_of
        .as_deref()
        .or(metadata.event_date.as_deref())
        .or(metadata.valid_from.as_deref())
        .and_then(parse_temporal_date)
        .or_else(|| {
            metadata_value(&metadata.body_text, &["as_of", "event_date", "valid_from"])
                .and_then(|value| parse_temporal_date(&value))
        })
}

fn temporal_sort_key(record: &TemporalFactRecord) -> (Option<TemporalDate>, Option<u64>, CellId) {
    (record.as_of, record.created_unix_seconds, record.cell_id)
}

fn metadata_value(body: &str, keys: &[&str]) -> Option<String> {
    body.lines().find_map(|line| {
        let (key, value) = line.split_once('=').or_else(|| line.split_once(':'))?;
        keys.iter()
            .any(|candidate| key.trim().eq_ignore_ascii_case(candidate))
            .then(|| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};

    use super::*;

    #[test]
    fn temporal_fact_index_marks_latest_and_superseded_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\ntype=fact\nas_of=2025-01-01\n\nproject=Apollo\nmetric=budget\nvalue=12000"
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=project:investments\nstatus=ready\ntype=fact\nas_of=2025-06-01\n\nproject=Apollo\nmetric=budget\nvalue=14000"
                .to_vec(),
        )
        .unwrap();

        let records = db.temporal_fact_index(&view("project:investments")).records;

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].cell_id, CellId(1));
        assert_eq!(records[0].superseded_by, Some(CellId(2)));
        assert!(!records[0].latest);
        assert_eq!(records[1].cell_id, CellId(2));
        assert!(records[1].latest);
        assert_eq!(records[1].value.as_deref(), Some("14000"));
    }

    #[test]
    fn temporal_fact_index_respects_agent_view_scope() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:visible\nstatus=ready\ntype=fact\nas_of=2025-01-01\n\nproject=Apollo\nmetric=budget\nvalue=12000"
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=tenant:hidden\nstatus=ready\ntype=fact\nas_of=2026-01-01\n\nproject=Apollo\nmetric=budget\nvalue=99999"
                .to_vec(),
        )
        .unwrap();

        let records = db.temporal_fact_index(&view("project:visible")).records;

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].cell_id, CellId(1));
    }

    fn view(scope: &str) -> AgentView {
        AgentView {
            agent_id: AgentId(1),
            label: None,
            readable_brains: BTreeSet::from([BrainId(1)]),
            readable_scopes: BTreeSet::from([scope_id(scope)]),
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
}
