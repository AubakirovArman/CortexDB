use cortex_storage::manifest::{ManifestHnswProfile, ManifestVectorProfile, StorageManifest};

use crate::error::{EngineError, EngineResult};
use crate::search::HnswBuildConfig;

pub(crate) fn manifest_hnsw_profile(config: HnswBuildConfig) -> EngineResult<ManifestHnswProfile> {
    let config = config.normalized();
    Ok(ManifestHnswProfile {
        max_neighbors: hnsw_profile_u32("max_neighbors", config.max_neighbors)?,
        ef_search: hnsw_profile_u32("ef_search", config.ef_search)?,
        layer_count: hnsw_profile_u32("layer_count", config.layer_count)?,
        metric: config.metric as u32,
        ef_construction: hnsw_profile_u32("ef_construction", config.ef_construction)?,
    })
}

pub(crate) fn ensure_checkpoint_profiles(
    manifest: &StorageManifest,
    hnsw_profile: Option<ManifestHnswProfile>,
    vector_profile: Option<ManifestVectorProfile>,
) -> EngineResult<()> {
    if manifest.live_segments.is_empty() {
        return Ok(());
    }
    if let (Some(existing), Some(next)) = (manifest.hnsw_profile, hnsw_profile) {
        if existing != next {
            return Err(EngineError::StorageInvariant(format!(
                "checkpoint hnsw profile {:?} does not match existing manifest profile {:?}; run compact with the new HNSW build config first",
                next, existing
            )));
        }
    }
    if let (Some(existing), Some(next)) = (manifest.vector_profile, vector_profile) {
        if existing != next {
            return Err(EngineError::StorageInvariant(format!(
                "checkpoint vector profile {:?} does not match existing manifest profile {:?}; run compact with a consistent vector collection profile first",
                next, existing
            )));
        }
    }
    Ok(())
}

fn hnsw_profile_u32(field: &'static str, value: usize) -> EngineResult<u32> {
    u32::try_from(value).map_err(|_| EngineError::HnswBuildConfigOutOfRange { field, value })
}
