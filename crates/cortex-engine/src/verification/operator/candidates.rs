use std::collections::BTreeSet;

use cortex_aql::AgentView;
use cortex_core::memtable::{CellVersion, ReadTxn};

use super::super::evidence::{version_contains_any_term, MaterializedVersion};
use super::super::VerificationEvidence;
use crate::database::Database;
use crate::error::EngineResult;
use crate::options::PayloadResidency;
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;

impl Database {
    pub(super) fn verification_candidate_versions<'a>(
        &'a self,
        fact: &str,
        txn: ReadTxn,
    ) -> EngineResult<Vec<&'a CellVersion>> {
        let fact_terms = tokenize(fact);
        if fact_terms.is_empty() {
            return Ok(self.memtable.visible_iter(txn).collect());
        }

        let mut cell_ids = BTreeSet::new();
        if !self.manifest().live_segments.is_empty() {
            let persisted = self.persisted_index_state_cached()?;
            for term in &fact_terms {
                if let Some(term_candidates) = persisted.lexical.terms.get(term) {
                    cell_ids.extend(
                        term_candidates
                            .iter()
                            .filter_map(|candidate| persisted.candidate_to_cell.get(candidate))
                            .copied(),
                    );
                }
            }
        }

        let checkpoint_seq = cortex_core::CommitSeq(self.manifest().checkpoint_seq);
        let changed = self
            .memtable
            .changed_cell_ids_after(checkpoint_seq)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let scan_all_live = self.manifest().live_segments.is_empty();
        if scan_all_live {
            for version in self.memtable.visible_iter(txn) {
                if version_contains_any_term(version, &fact_terms) {
                    cell_ids.insert(version.cell_id);
                }
            }
        } else {
            for cell_id in changed {
                let Some(version) = self.memtable.read(txn, cell_id) else {
                    continue;
                };
                if version_contains_any_term(version, &fact_terms) {
                    cell_ids.insert(version.cell_id);
                }
            }
        }

        if cell_ids.is_empty() && scan_all_live {
            return Ok(self.memtable.visible_iter(txn).collect());
        }
        if cell_ids.is_empty() {
            for version in self.memtable.visible_iter(txn).take(32) {
                cell_ids.insert(version.cell_id);
            }
        }

        let mut versions = cell_ids
            .into_iter()
            .filter_map(|cell_id| self.memtable.read(txn, cell_id))
            .collect::<Vec<_>>();
        versions.sort_by_key(|version| version.cell_id);
        versions.dedup_by_key(|version| version.cell_id);
        Ok(versions)
    }

    pub(super) fn filter_verification_candidate_permissions<'a>(
        &self,
        versions: Vec<&'a CellVersion>,
        view: &AgentView,
    ) -> Vec<&'a CellVersion> {
        versions
            .into_iter()
            .filter(|version| {
                let metadata = CellMetadata::from_version(version);
                view.can_read_scope(scope_id(&metadata.scope))
            })
            .collect()
    }

    pub(super) fn materialize_verification_versions<'a>(
        &'a self,
        versions: Vec<&'a CellVersion>,
    ) -> EngineResult<Vec<MaterializedVersion<'a>>> {
        versions
            .into_iter()
            .map(|version| {
                self.payload_for_version(version)
                    .map(|payload| MaterializedVersion { version, payload })
            })
            .collect()
    }

    pub(super) fn verification_source_support_versions<'a>(
        &'a self,
        evidence: &[VerificationEvidence],
        view: &AgentView,
        txn: ReadTxn,
    ) -> EngineResult<Vec<MaterializedVersion<'a>>> {
        if evidence.is_empty() {
            return Ok(Vec::new());
        }
        let evidence_ids = evidence
            .iter()
            .map(|item| item.cell_id)
            .map(|cell_id| format!("cell:{}", cell_id.0))
            .collect::<Vec<_>>();
        let checkpoint_seq = cortex_core::CommitSeq(self.manifest().checkpoint_seq);
        let scan_all_live = self.manifest().live_segments.is_empty()
            || self.payload_residency == PayloadResidency::Lazy;
        let visible = if scan_all_live {
            self.memtable.visible_iter(txn).collect::<Vec<_>>()
        } else {
            self.memtable
                .visible_created_after_iter(txn, checkpoint_seq)
                .collect::<Vec<_>>()
        };
        let mut support_versions = Vec::new();
        for version in visible {
            let metadata = CellMetadata::from_version(version);
            if metadata.cell_type != "relation" {
                continue;
            }
            if !view.can_read_scope(scope_id(&metadata.scope)) {
                continue;
            }
            let payload = self.payload_for_version(version)?;
            let is_source_support = evidence_ids.iter().any(|id| {
                std::str::from_utf8(&payload)
                    .map(|payload| payload.contains(id))
                    .unwrap_or(false)
            });
            if is_source_support {
                support_versions.push(MaterializedVersion { version, payload });
            }
        }
        Ok(support_versions)
    }
}
