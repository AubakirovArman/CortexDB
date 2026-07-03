use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{AgentId, AgentView, BindError, PolicyError, Q16, Q16_ONE};
use cortex_core::{CellDescriptor, CellId, CommitSeq, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCompressionSourceRef {
    pub source_cell_id: CellId,
    pub source_byte_start: usize,
    pub source_byte_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCompressionRequest {
    pub agent_id: AgentId,
    pub scope: String,
    pub summary_cell_id: Option<CellId>,
    pub summary_payload: Vec<u8>,
    pub source_refs: Vec<SemanticCompressionSourceRef>,
    pub answerability_q16: u16,
    pub external_worker: String,
    pub idempotency_key: Option<String>,
}

/// B4.1: the memory class of a cell for consolidation purposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryClass {
    /// Session/observational memory that decays and is a consolidation source:
    /// untyped session memory plus `observation` / `workflow_result` / `error_log`.
    Episodic,
    /// Durable memory that is never auto-consolidated away: `decision` / `preference`.
    Semantic,
}

/// B4.1: classify a cell's memory class. Pure and deterministic. Returns `None`
/// for non-memory cells and for memory cells whose `memory_type` is an unknown
/// explicit subtype (unclassified — never a consolidation candidate).
pub fn memory_class(descriptor: &CellDescriptor) -> Option<MemoryClass> {
    if descriptor.cell_type != KnowledgeCellType::Memory {
        return None;
    }
    match descriptor.memory_type.as_deref() {
        // Untyped memory is treated as an episodic session cell.
        None => Some(MemoryClass::Episodic),
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "decision" | "preference" => Some(MemoryClass::Semantic),
            "observation" | "workflow_result" | "workflowresult" | "error_log" | "errorlog" => {
                Some(MemoryClass::Episodic)
            }
            _ => None,
        },
    }
}

/// B4.2: a single episodic cell eligible for semantic consolidation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticCompressionCandidate {
    pub cell_id: CellId,
    pub freshness_q16: Q16,
}

/// B4.2: a deterministic group of consolidation candidates sharing an episodic
/// subtype within one scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCompressionCandidateGroup {
    pub scope: String,
    /// The episodic subtype key (`None` = untyped session memory).
    pub memory_type: Option<String>,
    pub candidates: Vec<SemanticCompressionCandidate>,
}

/// B4.5: the resolved provenance of a semantic-summary cell — which of its
/// declared `compression_source_cells` are still present and readable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompressionSourcesReport {
    pub summary_cell_id: CellId,
    /// Sources that are still present in the store (declaration order).
    pub source_cell_ids: Vec<CellId>,
    /// Sources referenced by the summary but no longer present.
    pub missing_source_cell_ids: Vec<CellId>,
    pub all_sources_present: bool,
}

/// B4.5 (retire): one episodic source cell that was demoted after consolidation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetiredSource {
    pub cell_id: CellId,
    /// The new `ttl_seconds` written so the cell expires `demote_ttl` from `now`.
    pub new_ttl_seconds: u64,
}

/// B4.5 (retire): the result of demoting a summary's episodic sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetireCompressionReport {
    pub summary_cell_id: CellId,
    /// Episodic sources whose TTL was shortened to expire `demote_ttl` from now.
    pub retired: Vec<RetiredSource>,
    /// Sources left untouched because they are semantic (never auto-retired), no
    /// longer present, or already expire at or before the demote horizon.
    pub skipped: Vec<CellId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCompressionReport {
    pub agent_id: AgentId,
    pub scope: String,
    pub summary_cell_id: CellId,
    pub committed_seq: CommitSeq,
    pub source_cell_ids: Vec<CellId>,
    pub source_ref_count: usize,
    pub answerability_q16: u16,
    pub provenance_preserved: bool,
    pub auditable: bool,
    pub external_worker: String,
    pub idempotency_key: Option<String>,
}

impl Database {
    pub fn commit_semantic_memory_compression(
        &mut self,
        view: &AgentView,
        request: SemanticCompressionRequest,
    ) -> EngineResult<SemanticCompressionReport> {
        if !self.semantic_compression.enabled {
            return Err(EngineError::FeatureDisabled("semantic_compression"));
        }
        validate_view(view, &request)?;
        self.validate_semantic_compression_request(view, &request)?;

        let summary_cell_id = match request.summary_cell_id {
            Some(cell_id) => cell_id,
            None => self.allocate_cell_id()?,
        };
        let committed_seq = self.put_cell(summary_cell_id, request.summary_payload.clone())?;
        let source_cell_ids = unique_source_cell_ids(&request.source_refs);
        let metadata = CellMetadata::from_payload(&request.summary_payload);
        let provenance_preserved = provenance_preserved(&metadata, &source_cell_ids);
        let auditable = provenance_preserved
            && metadata.compression_answerability_q16 == Some(request.answerability_q16)
            && metadata.compression_worker.as_deref() == Some(request.external_worker.as_str());

        Ok(SemanticCompressionReport {
            agent_id: request.agent_id,
            scope: request.scope,
            summary_cell_id,
            committed_seq,
            source_cell_ids,
            source_ref_count: request.source_refs.len(),
            answerability_q16: request.answerability_q16,
            provenance_preserved,
            auditable,
            external_worker: request.external_worker,
            idempotency_key: request.idempotency_key,
        })
    }

    /// B4.2: deterministically select readable episodic cells in `scope` that have
    /// decayed below `freshness_below_q16` and are not already a compression summary
    /// or a cell already consolidated into one. Grouped by episodic subtype, capped
    /// at `max_groups`. Read-only and behind the default-off feature flag, so it can
    /// never change default behaviour or goldens.
    ///
    /// This is the selection half that the existing
    /// [`Database::commit_semantic_memory_compression`] commit half consumes: the
    /// consolidation worker (B4.4) asks here for what to summarize, then commits.
    pub fn semantic_compression_candidates(
        &self,
        view: &AgentView,
        scope: &str,
        now_unix_seconds: u64,
        freshness_below_q16: Q16,
        max_groups: usize,
    ) -> EngineResult<Vec<SemanticCompressionCandidateGroup>> {
        if !self.semantic_compression.enabled {
            return Err(EngineError::FeatureDisabled("semantic_compression"));
        }
        if !PolicyRewrite::allows_scope(view, scope_id(scope)) {
            return Err(EngineError::AqlBind(BindError::PolicyDenied(
                PolicyError::ScopeNotReadable,
            )));
        }
        if max_groups == 0 {
            return Ok(Vec::new());
        }

        let freshness: BTreeMap<CellId, Q16> = self
            .memory_decay_scores(now_unix_seconds)
            .into_iter()
            .map(|score| (score.cell_id, score.freshness_q16))
            .collect();

        // Read each visible memory cell's payload once: collect the set of cells
        // already consolidated into a summary, and the per-cell metadata.
        let txn = self.read_txn();
        let memory_cells: Vec<(CellId, CellDescriptor)> = self
            .memtable
            .visible_iter(txn)
            .filter(|version| version.descriptor.cell_type == KnowledgeCellType::Memory)
            .map(|version| (version.cell_id, version.descriptor.clone()))
            .collect();

        let mut already_sourced: BTreeSet<CellId> = BTreeSet::new();
        let mut enriched: Vec<(CellId, CellDescriptor, bool)> =
            Vec::with_capacity(memory_cells.len());
        for (cell_id, descriptor) in memory_cells {
            let Some(payload) = self.get_cell(txn, cell_id) else {
                continue;
            };
            let metadata = CellMetadata::from_payload(&payload);
            let is_summary = metadata.compression_kind.is_some();
            if is_summary {
                for source in metadata.compression_source_cells {
                    already_sourced.insert(source);
                }
            }
            enriched.push((cell_id, descriptor, is_summary));
        }

        let mut grouped: BTreeMap<String, Vec<SemanticCompressionCandidate>> = BTreeMap::new();
        for (cell_id, descriptor, is_summary) in &enriched {
            if descriptor.scope != scope {
                continue;
            }
            if *is_summary || already_sourced.contains(cell_id) {
                continue;
            }
            if memory_class(descriptor) != Some(MemoryClass::Episodic) {
                continue;
            }
            let fresh = freshness.get(cell_id).copied().unwrap_or(Q16_ONE);
            if fresh >= freshness_below_q16 {
                continue;
            }
            let key = descriptor.memory_type.clone().unwrap_or_default();
            grouped
                .entry(key)
                .or_default()
                .push(SemanticCompressionCandidate {
                    cell_id: *cell_id,
                    freshness_q16: fresh,
                });
        }

        let mut groups: Vec<SemanticCompressionCandidateGroup> = grouped
            .into_iter()
            .map(|(key, mut candidates)| {
                // Stalest first, then by cell id — fully deterministic.
                candidates.sort_by(|a, b| {
                    a.freshness_q16
                        .cmp(&b.freshness_q16)
                        .then(a.cell_id.cmp(&b.cell_id))
                });
                SemanticCompressionCandidateGroup {
                    scope: scope.to_owned(),
                    memory_type: if key.is_empty() { None } else { Some(key) },
                    candidates,
                }
            })
            .collect();
        groups.truncate(max_groups);
        Ok(groups)
    }

    /// B4.5 (unfold read): resolve a semantic summary's provenance — the source
    /// cells it consolidated — fail-closed on readable scope. The summary itself
    /// must be a readable `semantic_summary`; any source in a scope this view
    /// cannot read is a hard `ScopeNotReadable` (never a silent drop). Sources
    /// that have since been removed are reported as missing rather than an error.
    /// Read-only and behind the default-off feature flag.
    pub fn compression_sources(
        &self,
        view: &AgentView,
        summary_cell_id: CellId,
    ) -> EngineResult<CompressionSourcesReport> {
        if !self.semantic_compression.enabled {
            return Err(EngineError::FeatureDisabled("semantic_compression"));
        }
        let (payload, descriptor) = self
            .get_latest_cell_with_descriptor(summary_cell_id)
            .ok_or(cortex_core::CoreError::CellNotFound(summary_cell_id))?;
        if !PolicyRewrite::allows_scope(view, scope_id(&descriptor.scope)) {
            return Err(EngineError::AqlBind(BindError::PolicyDenied(
                PolicyError::ScopeNotReadable,
            )));
        }
        let metadata = CellMetadata::from_payload(&payload);
        if metadata.compression_kind.as_deref() != Some("semantic_summary") {
            return invalid("cell is not a semantic summary");
        }

        let mut present = Vec::new();
        let mut missing = Vec::new();
        for source in &metadata.compression_source_cells {
            match self.get_latest_cell_descriptor(*source) {
                Some(source_descriptor) => {
                    // Fail-closed: an unreadable source scope aborts the resolve.
                    if !PolicyRewrite::allows_scope(view, scope_id(&source_descriptor.scope)) {
                        return Err(EngineError::AqlBind(BindError::PolicyDenied(
                            PolicyError::ScopeNotReadable,
                        )));
                    }
                    present.push(*source);
                }
                None => missing.push(*source),
            }
        }

        Ok(CompressionSourcesReport {
            summary_cell_id,
            all_sources_present: missing.is_empty(),
            source_cell_ids: present,
            missing_source_cell_ids: missing,
        })
    }

    /// B4.5 (retire): demote a summary's EPISODIC source cells by shortening their
    /// TTL so they expire `demote_ttl_seconds` from `now` — consolidated sources
    /// age out naturally instead of lingering. Never tombstones, never touches
    /// semantic sources, and requires the summary readable + each demoted source
    /// writable. Read-modify-write of an existing metadata field (no new canonical
    /// fields), behind the default-off feature flag → golden-safe.
    pub fn retire_compression_sources(
        &mut self,
        view: &AgentView,
        summary_cell_id: CellId,
        demote_ttl_seconds: u64,
        now_unix_seconds: u64,
    ) -> EngineResult<RetireCompressionReport> {
        if !self.semantic_compression.enabled {
            return Err(EngineError::FeatureDisabled("semantic_compression"));
        }
        let (payload, descriptor) = self
            .get_latest_cell_with_descriptor(summary_cell_id)
            .ok_or(cortex_core::CoreError::CellNotFound(summary_cell_id))?;
        if !PolicyRewrite::allows_scope(view, scope_id(&descriptor.scope)) {
            return Err(EngineError::AqlBind(BindError::PolicyDenied(
                PolicyError::ScopeNotReadable,
            )));
        }
        let metadata = CellMetadata::from_payload(&payload);
        if metadata.compression_kind.as_deref() != Some("semantic_summary") {
            return invalid("cell is not a semantic summary");
        }

        // Read-only pass: decide which sources to demote (and to what TTL).
        let target_expiry = now_unix_seconds.saturating_add(demote_ttl_seconds);
        let mut demotions: Vec<(CellId, Vec<u8>, u64, u64)> = Vec::new();
        let mut skipped = Vec::new();
        for source in &metadata.compression_source_cells {
            let Some((source_payload, source_descriptor)) =
                self.get_latest_cell_with_descriptor(*source)
            else {
                skipped.push(*source);
                continue;
            };
            if memory_class(&source_descriptor) != Some(MemoryClass::Episodic) {
                skipped.push(*source);
                continue;
            }
            if !view.can_write_scope(scope_id(&source_descriptor.scope)) {
                return Err(EngineError::AqlBind(BindError::PolicyDenied(
                    PolicyError::ScopeNotWritable,
                )));
            }
            let current_expiry = source_descriptor
                .created_unix_seconds
                .zip(source_descriptor.ttl_seconds)
                .map(|(created, ttl)| created.saturating_add(ttl));
            // Only demote sources that would otherwise outlive the demote horizon.
            if current_expiry.is_some_and(|expiry| expiry <= target_expiry) {
                skipped.push(*source);
                continue;
            }
            let created = source_descriptor
                .created_unix_seconds
                .unwrap_or(now_unix_seconds);
            let new_ttl = now_unix_seconds
                .saturating_sub(created)
                .saturating_add(demote_ttl_seconds);
            demotions.push((*source, source_payload, created, new_ttl));
        }

        // Write pass: rewrite each demoted source's TTL, preserving body + all other
        // metadata (created is kept, so provenance is intact).
        let mut retired = Vec::new();
        for (cell_id, source_payload, created, new_ttl) in demotions {
            let new_payload = rewrite_ttl(&source_payload, created, new_ttl);
            self.put_cell(cell_id, new_payload)?;
            retired.push(RetiredSource {
                cell_id,
                new_ttl_seconds: new_ttl,
            });
        }

        Ok(RetireCompressionReport {
            summary_cell_id,
            retired,
            skipped,
        })
    }

    fn validate_semantic_compression_request(
        &self,
        view: &AgentView,
        request: &SemanticCompressionRequest,
    ) -> EngineResult<()> {
        if request.external_worker.trim().is_empty() {
            return invalid("external_worker is required");
        }
        if request.summary_payload.is_empty() {
            return invalid("summary_payload is required");
        }
        if request.source_refs.is_empty() {
            return invalid("source_refs are required");
        }
        if request.answerability_q16 < self.semantic_compression.min_answerability_q16 {
            return invalid("answerability_q16 is below semantic compression threshold");
        }
        validate_source_refs(&request.source_refs)?;
        self.validate_source_cells_are_readable(view, &request.source_refs)?;
        validate_summary_payload(request)
    }

    fn validate_source_cells_are_readable(
        &self,
        view: &AgentView,
        source_refs: &[SemanticCompressionSourceRef],
    ) -> EngineResult<()> {
        for cell_id in unique_source_cell_ids(source_refs) {
            let descriptor = self
                .memtable
                .read(self.read_txn(), cell_id)
                .map(|version| &version.descriptor)
                .ok_or(cortex_core::CoreError::CellNotFound(cell_id))?;
            if !PolicyRewrite::allows_scope(view, scope_id(&descriptor.scope)) {
                return Err(EngineError::AqlBind(BindError::PolicyDenied(
                    PolicyError::ScopeNotReadable,
                )));
            }
        }
        Ok(())
    }
}

fn validate_view(view: &AgentView, request: &SemanticCompressionRequest) -> EngineResult<()> {
    if view.agent_id != request.agent_id {
        return invalid("semantic compression agent_id does not match AgentView");
    }
    if !view.can_write_scope(scope_id(&request.scope)) {
        return Err(EngineError::AqlBind(BindError::PolicyDenied(
            PolicyError::ScopeNotWritable,
        )));
    }
    Ok(())
}

fn validate_source_refs(source_refs: &[SemanticCompressionSourceRef]) -> EngineResult<()> {
    let mut seen = BTreeSet::new();
    for source_ref in source_refs {
        if source_ref.source_byte_start > source_ref.source_byte_end {
            return invalid("source_ref byte range is invalid");
        }
        seen.insert(source_ref.source_cell_id);
    }
    if seen.is_empty() {
        return invalid("source_refs do not contain source cells");
    }
    Ok(())
}

fn validate_summary_payload(request: &SemanticCompressionRequest) -> EngineResult<()> {
    let descriptor = CellDescriptor::from_payload_lossy(&request.summary_payload);
    if descriptor.scope != request.scope {
        return invalid("summary_payload scope does not match request scope");
    }
    if descriptor.cell_type != KnowledgeCellType::Memory {
        return invalid("summary_payload must be type=memory");
    }
    let metadata = CellMetadata::from_payload(&request.summary_payload);
    if metadata.compression_kind.as_deref() != Some("semantic_summary") {
        return invalid("summary_payload must set compression_kind=semantic_summary");
    }
    if metadata.compression_answerability_q16 != Some(request.answerability_q16) {
        return invalid("summary_payload answerability metadata does not match request");
    }
    if metadata.compression_worker.as_deref() != Some(request.external_worker.as_str()) {
        return invalid("summary_payload worker metadata does not match request");
    }
    let source_cell_ids = unique_source_cell_ids(&request.source_refs);
    if !provenance_preserved(&metadata, &source_cell_ids) {
        return invalid("summary_payload compression_source_cells does not match source_refs");
    }
    Ok(())
}

fn unique_source_cell_ids(source_refs: &[SemanticCompressionSourceRef]) -> Vec<CellId> {
    source_refs
        .iter()
        .map(|source_ref| source_ref.source_cell_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn provenance_preserved(metadata: &CellMetadata, source_cell_ids: &[CellId]) -> bool {
    metadata.compression_source_cells == source_cell_ids
}

/// Rewrite a cell payload's `created_unix_seconds` + `ttl_seconds` header lines,
/// preserving every other header line and the body exactly (cell metadata is
/// order-independent key=value, so re-appending these two lines is safe).
fn rewrite_ttl(payload: &[u8], created: u64, new_ttl: u64) -> Vec<u8> {
    let text = String::from_utf8_lossy(payload);
    let (header, body) = text
        .split_once("\n\n")
        .map(|(h, b)| (h, Some(b)))
        .unwrap_or((text.as_ref(), None));
    let mut lines: Vec<String> = header
        .lines()
        .filter(|line| {
            !line.starts_with("ttl_seconds=") && !line.starts_with("created_unix_seconds=")
        })
        .map(str::to_owned)
        .collect();
    lines.push(format!("created_unix_seconds={created}"));
    lines.push(format!("ttl_seconds={new_ttl}"));
    let header = lines.join("\n");
    match body {
        Some(body) => format!("{header}\n\n{body}").into_bytes(),
        None => header.into_bytes(),
    }
}

fn invalid<T>(message: &str) -> EngineResult<T> {
    Err(EngineError::InvalidSemanticCompression(message.to_owned()))
}

#[cfg(test)]
mod memory_class_tests {
    use super::{memory_class, MemoryClass};
    use cortex_core::{CellDescriptor, KnowledgeCellType};

    fn memory(memory_type: Option<&str>) -> CellDescriptor {
        CellDescriptor {
            cell_type: KnowledgeCellType::Memory,
            memory_type: memory_type.map(str::to_owned),
            ..CellDescriptor::default()
        }
    }

    #[test]
    fn decision_and_preference_are_semantic() {
        assert_eq!(
            memory_class(&memory(Some("decision"))),
            Some(MemoryClass::Semantic)
        );
        assert_eq!(
            memory_class(&memory(Some("preference"))),
            Some(MemoryClass::Semantic)
        );
        // Case-insensitive, matching MemoryType::from_str.
        assert_eq!(
            memory_class(&memory(Some("Decision"))),
            Some(MemoryClass::Semantic)
        );
    }

    #[test]
    fn observations_workflow_errors_and_untyped_are_episodic() {
        for raw in [
            "observation",
            "workflow_result",
            "workflowresult",
            "error_log",
            "errorlog",
        ] {
            assert_eq!(
                memory_class(&memory(Some(raw))),
                Some(MemoryClass::Episodic),
                "{raw} should be episodic"
            );
        }
        // Untyped memory is an episodic session cell.
        assert_eq!(memory_class(&memory(None)), Some(MemoryClass::Episodic));
    }

    #[test]
    fn non_memory_and_unknown_subtypes_are_unclassified() {
        // Not a memory cell.
        let fact = CellDescriptor {
            cell_type: KnowledgeCellType::Fact,
            memory_type: Some("decision".to_owned()),
            ..CellDescriptor::default()
        };
        assert_eq!(memory_class(&fact), None);
        // Unknown explicit subtype: never a consolidation candidate.
        assert_eq!(memory_class(&memory(Some("mystery"))), None);
    }
}
