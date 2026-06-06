use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::database::Database;
use crate::error::EngineResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentBundle {
    pub segment_id: u64,
    pub segment_path: PathBuf,
    pub bitmap_path: PathBuf,
    pub lexical_path: PathBuf,
    pub vector_path: PathBuf,
    pub hnsw_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetiredSegmentGc {
    pub retired_segments_removed: usize,
    pub files_removed: usize,
}

impl SegmentBundle {
    pub fn new(root: &Path, segment_id: u64) -> Self {
        Self {
            segment_id,
            segment_path: root.join(format!("segment-{segment_id}.acs")),
            bitmap_path: root.join(format!("segment-{segment_id}.acb")),
            lexical_path: root.join(format!("segment-{segment_id}.aci")),
            vector_path: root.join(format!("segment-{segment_id}.acv")),
            hnsw_path: root.join(format!("segment-{segment_id}.ach")),
        }
    }

    pub fn exists_all(&self) -> bool {
        self.segment_path.exists()
            && self.bitmap_path.exists()
            && self.lexical_path.exists()
            && self.vector_path.exists()
    }

    pub fn remove_files(&self) -> EngineResult<usize> {
        let mut removed = 0;
        for path in [
            &self.segment_path,
            &self.bitmap_path,
            &self.lexical_path,
            &self.vector_path,
            &self.hnsw_path,
        ] {
            match fs::remove_file(path) {
                Ok(()) => removed += 1,
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(removed)
    }
}

impl Database {
    pub fn live_segment_bundles(&self) -> Vec<SegmentBundle> {
        self.manifest
            .live_segments
            .iter()
            .map(|segment| SegmentBundle::new(&self.segments_path, segment.id))
            .collect()
    }

    pub fn retired_segment_bundles(&self) -> Vec<SegmentBundle> {
        self.manifest
            .retired_segments
            .iter()
            .map(|segment| SegmentBundle::new(&self.segments_path, segment.id))
            .collect()
    }

    pub fn garbage_collect_retired_segments(&mut self) -> EngineResult<RetiredSegmentGc> {
        let retired = self.retired_segment_bundles();
        let mut files_removed = 0;
        for bundle in &retired {
            files_removed += bundle.remove_files()?;
        }
        let retired_segments_removed = self.manifest.retired_segments.len();
        if retired_segments_removed > 0 {
            self.manifest.retired_segments.clear();
            self.manifest.generation += 1;
            self.manifest.store(&self.manifest_path)?;
        }
        Ok(RetiredSegmentGc {
            retired_segments_removed,
            files_removed,
        })
    }
}
