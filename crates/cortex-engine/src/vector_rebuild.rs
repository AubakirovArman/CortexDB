use cortex_storage::manifest::{ManifestHnswProfile, ManifestVectorProfile};
use cortex_storage::segment::SegmentReader;

use crate::checkpoint::{
    hnsw, hnsw_path, manifest_hnsw_profile, segment_path, vector, vector_path,
};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::search::{DistanceMetric, HnswBuildConfig};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VectorRebuildReport {
    pub segments_checked: usize,
    pub cells_scanned: usize,
    pub vector_candidates: usize,
    pub vector_indexes_rebuilt: usize,
    pub hnsw_graphs_rebuilt: usize,
    pub hnsw_enabled: bool,
}

impl Database {
    pub fn rebuild_vector_indexes(
        &mut self,
        rebuild_hnsw: bool,
    ) -> EngineResult<VectorRebuildReport> {
        std::fs::create_dir_all(&self.segments_path)?;
        let config = rebuild_config(
            self.hnsw_build_config,
            self.manifest.hnsw_profile,
            self.manifest.vector_profile,
        );
        let hnsw_enabled = rebuild_hnsw || self.manifest.hnsw_profile.is_some();
        let mut vector_profile = self.manifest.vector_profile;
        let mut hnsw_profile = self.manifest.hnsw_profile;
        if hnsw_enabled && hnsw_profile.is_none() {
            hnsw_profile = Some(manifest_hnsw_profile(config)?);
        }

        let mut report = VectorRebuildReport {
            hnsw_enabled,
            ..VectorRebuildReport::default()
        };

        for segment in &self.manifest.live_segments {
            let cells = SegmentReader::read(segment_path(&self.segments_path, segment.id))?;
            report.segments_checked += 1;
            report.cells_scanned += cells.len();

            let segment_vector_profile = vector::vector_profile_for_cells(&cells, config)?;
            merge_vector_profile(&mut vector_profile, segment_vector_profile, segment.id)?;

            let vector_index = vector::vector_index_for_cells(&cells);
            report.vector_candidates += vector_index.vectors.len();
            vector_index.write(vector_path(&self.segments_path, segment.id))?;
            report.vector_indexes_rebuilt += 1;

            if hnsw_enabled {
                hnsw::hnsw_graph_for_cells_with_config(&cells, config)?
                    .write(hnsw_path(&self.segments_path, segment.id))?;
                report.hnsw_graphs_rebuilt += 1;
            }
        }

        if self.manifest.vector_profile != vector_profile
            || self.manifest.hnsw_profile != hnsw_profile
        {
            self.manifest.vector_profile = vector_profile;
            self.manifest.hnsw_profile = hnsw_profile;
            self.manifest.store(&self.manifest_path)?;
        }

        self.validate_storage()?;
        Ok(report)
    }
}

fn rebuild_config(
    default: HnswBuildConfig,
    hnsw_profile: Option<ManifestHnswProfile>,
    vector_profile: Option<ManifestVectorProfile>,
) -> HnswBuildConfig {
    if let Some(profile) = hnsw_profile {
        return HnswBuildConfig {
            max_neighbors: profile.max_neighbors as usize,
            ef_search: profile.ef_search as usize,
            layer_count: profile.layer_count as usize,
            ef_construction: profile.ef_construction as usize,
            metric: metric_from_manifest(profile.metric),
        }
        .normalized();
    }
    let mut config = default.normalized();
    if let Some(profile) = vector_profile {
        config.metric = metric_from_manifest(profile.metric);
    }
    config
}

fn metric_from_manifest(metric: u32) -> DistanceMetric {
    match metric {
        1 => DistanceMetric::Cosine,
        2 => DistanceMetric::L2,
        _ => DistanceMetric::DotProduct,
    }
}

fn merge_vector_profile(
    current: &mut Option<ManifestVectorProfile>,
    next: Option<ManifestVectorProfile>,
    segment_id: u64,
) -> EngineResult<()> {
    let Some(next) = next else {
        return Ok(());
    };
    match current {
        Some(existing) if *existing != next => Err(EngineError::StorageInvariant(format!(
            "segment {segment_id} vector profile {:?} does not match existing profile {:?}",
            next, existing
        ))),
        Some(_) => Ok(()),
        None => {
            *current = Some(next);
            Ok(())
        }
    }
}
