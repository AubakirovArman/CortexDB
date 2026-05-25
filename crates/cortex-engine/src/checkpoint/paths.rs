use std::path::{Path, PathBuf};

pub(crate) fn manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.acm")
}

pub(crate) fn segments_path(root: &Path) -> PathBuf {
    root.join("segments")
}

pub(crate) fn segment_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.acs"))
}

pub(crate) fn bitmap_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.acb"))
}

pub(crate) fn lexical_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.aci"))
}

pub(crate) fn vector_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.acv"))
}

pub(crate) fn hnsw_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.ach"))
}
