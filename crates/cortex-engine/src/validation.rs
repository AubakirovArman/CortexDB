use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use cortex_core::memtable::MemTableStats;
use cortex_core::CommitSeq;
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{
    ManifestHnswProfile, ManifestSegment, ManifestVectorProfile, StorageManifest,
};
use cortex_storage::segment::SegmentReader;
use cortex_storage::vectors::VectorIndex;
use cortex_storage::wal::{WalReader, WalWriterMetrics};

use crate::checkpoint::{bitmap_path, hnsw_path, lexical_path, segment_path, vector_path};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::memory_accounting::estimate_database_memory;
use crate::search::HnswIndex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageStats {
    pub current_seq: CommitSeq,
    pub checkpoint_seq: CommitSeq,
    pub live_segments: usize,
    pub retired_segments: usize,
    pub memtable: MemTableStats,
    pub memtable_payload_bytes: usize,
    pub estimated_memtable_bytes: usize,
    pub estimated_index_bytes: usize,
    pub estimated_context_pack_bytes: usize,
    pub estimated_total_memory_bytes: usize,
    pub live_segment_bytes: u64,
    pub retired_segment_bytes: u64,
    pub total_segment_bytes: u64,
    pub durable_storage_bytes: u64,
    pub live_segment_payload_bytes: u64,
    pub logical_payload_bytes: u64,
    pub space_amplification_q16: u32,
    pub write_amplification_q16: u32,
    pub compaction_pressure_q16: u32,
    pub wal_size_bytes: u64,
    pub wal_writer: WalWriterMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageValidation {
    pub live_segments_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: usize,
    pub wal_safe_truncate_offset: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageValidationReport {
    pub manifest_ok: bool,
    pub wal_ok: bool,
    pub live_segments_checked: usize,
    pub bitmap_indexes_checked: usize,
    pub lexical_indexes_checked: usize,
    pub vector_indexes_checked: usize,
    pub hnsw_graphs_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: usize,
    pub wal_safe_truncate_offset: u64,
    pub errors: Vec<String>,
}

impl Database {
    pub fn storage_stats(&self) -> EngineResult<StorageStats> {
        let memory = estimate_database_memory(self)?;
        let live_usage = segment_usage(&self.segments_path, &self.manifest.live_segments, true)?;
        let retired_usage =
            segment_usage(&self.segments_path, &self.manifest.retired_segments, false)?;
        let total_segment_bytes = live_usage
            .bundle_bytes
            .saturating_add(retired_usage.bundle_bytes);
        let wal_size_bytes = file_len_or_zero(&self.wal_path)?;
        let wal_writer = self.writer.metrics()?;
        let durable_storage_bytes = total_segment_bytes.saturating_add(wal_size_bytes);
        let logical_payload_bytes = live_usage
            .payload_bytes
            .max(memory.memtable_payload_bytes as u64);
        let write_bytes = total_segment_bytes.saturating_add(wal_writer.bytes_written);
        Ok(StorageStats {
            current_seq: self.current_seq,
            checkpoint_seq: CommitSeq(self.manifest.checkpoint_seq),
            live_segments: self.manifest.live_segments.len(),
            retired_segments: self.manifest.retired_segments.len(),
            memtable: self.memtable.stats(),
            memtable_payload_bytes: memory.memtable_payload_bytes,
            estimated_memtable_bytes: memory.estimated_memtable_bytes,
            estimated_index_bytes: memory.estimated_index_bytes,
            estimated_context_pack_bytes: memory.estimated_context_pack_bytes,
            estimated_total_memory_bytes: memory.estimated_total_memory_bytes,
            live_segment_bytes: live_usage.bundle_bytes,
            retired_segment_bytes: retired_usage.bundle_bytes,
            total_segment_bytes,
            durable_storage_bytes,
            live_segment_payload_bytes: live_usage.payload_bytes,
            logical_payload_bytes,
            space_amplification_q16: ratio_q16(durable_storage_bytes, logical_payload_bytes),
            write_amplification_q16: ratio_q16(write_bytes, logical_payload_bytes),
            compaction_pressure_q16: ratio_q16(retired_usage.bundle_bytes, total_segment_bytes),
            wal_size_bytes,
            wal_writer,
        })
    }

    pub fn validate_storage(&self) -> EngineResult<StorageValidation> {
        let report = self.validate_storage_report();
        if !report.errors.is_empty() {
            return Err(EngineError::StorageInvariant(report.errors.join("; ")));
        }
        Ok(StorageValidation {
            live_segments_checked: report.live_segments_checked,
            cells_checked: report.cells_checked,
            wal_records_checked: report.wal_records_checked,
            wal_safe_truncate_offset: report.wal_safe_truncate_offset,
        })
    }

    pub fn validate_storage_report(&self) -> StorageValidationReport {
        let mut report = StorageValidationReport::default();
        let manifest = match StorageManifest::load(&self.manifest_path) {
            Ok(manifest) => {
                report.manifest_ok = true;
                manifest
            }
            Err(error) => {
                report.errors.push(format!("manifest: {error}"));
                check_wal(&self.wal_path, &mut report);
                return report;
            }
        };
        let mut cells_checked = 0;
        let mut live_ids = BTreeSet::new();
        let mut retired_ids = BTreeSet::new();
        let mut candidates = BTreeMap::new();
        let mut hnsw_build_profile = None;
        let mut vector_collection_profile = None;
        let manifest_hnsw_profile = manifest.hnsw_profile.map(hnsw_manifest_profile_key);
        let manifest_vector_profile = manifest.vector_profile.map(vector_manifest_profile_key);
        for segment in &manifest.live_segments {
            if !live_ids.insert(segment.id) {
                report
                    .errors
                    .push(format!("duplicate live segment id: {}", segment.id));
            }
            if manifest.checkpoint_seq < segment.checkpoint_seq {
                report.errors.push(format!(
                    "manifest checkpoint_seq {} is behind segment {} checkpoint_seq {}",
                    manifest.checkpoint_seq, segment.id, segment.checkpoint_seq
                ));
            }
            let segment_file = segment_path(&self.segments_path, segment.id);
            if !segment_file.exists() {
                report
                    .errors
                    .push(format!("missing storage file: {}", segment_file.display()));
                continue;
            }
            let cells = match SegmentReader::read(&segment_file) {
                Ok(cells) => cells,
                Err(error) => {
                    report
                        .errors
                        .push(format!("segment {}: {error}", segment.id));
                    continue;
                }
            };
            report.live_segments_checked += 1;
            if cells.len() != segment.cell_count as usize {
                report.errors.push(format!(
                    "segment {} cell_count mismatch: manifest={} actual={}",
                    segment.id,
                    segment.cell_count,
                    cells.len()
                ));
            }
            if cells.iter().any(|cell| cell.candidate_id == 0) {
                report.errors.push("invalid candidate id: 0".to_owned());
            }
            for cell in &cells {
                if let Some(previous) = candidates.insert(cell.candidate_id, cell.cell_id) {
                    if previous != cell.cell_id {
                        report.errors.push(format!(
                            "candidate {} maps to multiple cells",
                            cell.candidate_id
                        ));
                    }
                }
            }
            match BitmapIndex::read(bitmap_path(&self.segments_path, segment.id)) {
                Ok(_) => report.bitmap_indexes_checked += 1,
                Err(error) => report
                    .errors
                    .push(format!("bitmap index {}: {error}", segment.id)),
            }
            match LexicalIndex::read(lexical_path(&self.segments_path, segment.id)) {
                Ok(_) => report.lexical_indexes_checked += 1,
                Err(error) => report
                    .errors
                    .push(format!("lexical index {}: {error}", segment.id)),
            }
            let vector_index = match VectorIndex::read(vector_path(&self.segments_path, segment.id))
            {
                Ok(index) => {
                    report.vector_indexes_checked += 1;
                    let dimension_report = index.dimension_report();
                    if !dimension_report.is_valid() {
                        report.errors.push(format!(
                            "vector index {} dimensions: {}",
                            segment.id,
                            dimension_report.summary()
                        ));
                    }
                    if let (Some(expected), Some(actual)) =
                        (manifest_vector_profile, dimension_report.expected_dimension)
                    {
                        if expected.0 as usize != actual {
                            report.errors.push(format!(
                                "vector collection {} profile dimension={} does not match vector index dimension={}",
                                segment.id, expected.0, actual
                            ));
                        }
                    }
                    Some(index)
                }
                Err(error) => {
                    report
                        .errors
                        .push(format!("vector index {}: {error}", segment.id));
                    None
                }
            };
            match HnswGraphIndex::read(hnsw_path(&self.segments_path, segment.id)) {
                Ok(graph) => {
                    report.hnsw_graphs_checked += 1;
                    let profile = hnsw_profile_key(&graph);
                    if let Some(expected) = manifest_hnsw_profile {
                        if profile != expected {
                            report.errors.push(format!(
                                "hnsw graph {} profile {} does not match manifest profile {}",
                                segment.id,
                                format_hnsw_profile(profile),
                                format_hnsw_profile(expected)
                            ));
                        }
                    }
                    if let Some(previous) = hnsw_build_profile {
                        if previous != profile {
                            report.errors.push(format!(
                                "mixed hnsw build profiles across live segments: segment {} has {} but earlier segment has {}",
                                segment.id,
                                format_hnsw_profile(profile),
                                format_hnsw_profile(previous)
                            ));
                        }
                    } else {
                        hnsw_build_profile = Some(profile);
                    }
                    if let Some(vector_index) = &vector_index {
                        if let Some(actual) = vector_profile_key(vector_index, &graph) {
                            if let Some(expected) = manifest_vector_profile {
                                if actual != expected {
                                    report.errors.push(format!(
                                        "vector collection {} profile {} does not match manifest profile {}",
                                        segment.id,
                                        format_vector_profile(actual),
                                        format_vector_profile(expected)
                                    ));
                                }
                            }
                            if let Some(previous) = vector_collection_profile {
                                if previous != actual {
                                    report.errors.push(format!(
                                        "mixed vector collection profiles across live segments: segment {} has {} but earlier segment has {}",
                                        segment.id,
                                        format_vector_profile(actual),
                                        format_vector_profile(previous)
                                    ));
                                }
                            } else {
                                vector_collection_profile = Some(actual);
                            }
                            if graph.dimension != 0 && graph.dimension != actual.0 {
                                report.errors.push(format!(
                                    "hnsw graph {} dimension {} does not match vector index dimension {}",
                                    segment.id, graph.dimension, actual.0
                                ));
                            }
                        }
                        let max_neighbors = graph.max_neighbors as usize;
                        let ef_search = graph.ef_search as usize;
                        let index = HnswIndex::from_graph(
                            vector_index.vectors.clone(),
                            graph,
                            max_neighbors,
                            ef_search,
                        );
                        let hnsw_report = index.integrity_report();
                        if !hnsw_report.is_valid() {
                            report.errors.push(format!(
                                "hnsw graph {} integrity: {}",
                                segment.id,
                                hnsw_report.summary()
                            ));
                        }
                    }
                }
                Err(error)
                    if manifest_hnsw_profile.is_none()
                        && matches!(error, cortex_storage::StorageError::Io(ref io) if io.kind() == ErrorKind::NotFound) =>
                    {}
                Err(error) => report
                    .errors
                    .push(format!("hnsw graph {}: {error}", segment.id)),
            }
            cells_checked += cells.len();
        }
        for segment in &manifest.retired_segments {
            if !retired_ids.insert(segment.id) || live_ids.contains(&segment.id) {
                report.errors.push(format!(
                    "retired segment {} conflicts with manifest references",
                    segment.id
                ));
            }
        }
        report.cells_checked = cells_checked;
        check_wal(&self.wal_path, &mut report);
        report
    }
}

fn check_wal(path: &std::path::Path, report: &mut StorageValidationReport) {
    match WalReader::scan_best_effort_path(path) {
        Ok(wal) => {
            report.wal_ok = true;
            report.wal_records_checked = wal.records.len();
            report.wal_safe_truncate_offset = wal.safe_truncate_offset;
        }
        Err(error) => report.errors.push(format!("wal: {error}")),
    }
}

fn hnsw_profile_key(graph: &HnswGraphIndex) -> (u32, u32, u32, u32, u32) {
    (
        graph.max_neighbors,
        graph.ef_search,
        graph.layer_count,
        u32::from(graph.metric),
        graph.ef_construction.max(graph.ef_search),
    )
}

fn hnsw_manifest_profile_key(profile: ManifestHnswProfile) -> (u32, u32, u32, u32, u32) {
    (
        profile.max_neighbors,
        profile.ef_search,
        profile.layer_count,
        profile.metric,
        profile.ef_construction.max(profile.ef_search),
    )
}

fn vector_manifest_profile_key(profile: ManifestVectorProfile) -> (u32, u32) {
    (profile.dimension, profile.metric)
}

fn vector_profile_key(index: &VectorIndex, graph: &HnswGraphIndex) -> Option<(u32, u32)> {
    let report = index.dimension_report();
    let dimension = report.expected_dimension?;
    let dimension = u32::try_from(dimension).ok()?;
    Some((dimension, u32::from(graph.metric)))
}

fn format_hnsw_profile(profile: (u32, u32, u32, u32, u32)) -> String {
    let (max_neighbors, ef_search, layer_count, metric, ef_construction) = profile;
    if profile == (0, 0, 0, 0, 0) {
        return "legacy/unknown".to_owned();
    }
    format!(
        "max_neighbors={max_neighbors},ef_search={ef_search},layer_count={layer_count},metric={metric},ef_construction={ef_construction}"
    )
}

fn format_vector_profile(profile: (u32, u32)) -> String {
    let (dimension, metric) = profile;
    format!("dimension={dimension},metric={metric}")
}

fn file_len_or_zero(path: &std::path::Path) -> EngineResult<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SegmentUsage {
    bundle_bytes: u64,
    payload_bytes: u64,
}

fn segment_usage(
    segments_path: &Path,
    segments: &[ManifestSegment],
    read_payloads: bool,
) -> EngineResult<SegmentUsage> {
    let mut usage = SegmentUsage::default();
    for segment in segments {
        let segment_file = segment_path(segments_path, segment.id);
        let bundle_paths = [
            segment_file.clone(),
            bitmap_path(segments_path, segment.id),
            lexical_path(segments_path, segment.id),
            vector_path(segments_path, segment.id),
            hnsw_path(segments_path, segment.id),
        ];
        for path in bundle_paths {
            usage.bundle_bytes = usage.bundle_bytes.saturating_add(file_len_or_zero(&path)?);
        }
        if read_payloads {
            for cell in SegmentReader::read(&segment_file)? {
                if cell.deleted_seq.is_none() {
                    usage = SegmentUsage {
                        bundle_bytes: usage.bundle_bytes,
                        payload_bytes: usage
                            .payload_bytes
                            .saturating_add(cell.payload.len() as u64),
                    };
                }
            }
        }
    }
    Ok(usage)
}

fn ratio_q16(numerator: u64, denominator: u64) -> u32 {
    if denominator == 0 {
        return 0;
    }
    let scaled = (u128::from(numerator) << 16) / u128::from(denominator);
    scaled.min(u128::from(u32::MAX)) as u32
}
