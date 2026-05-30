use cortex_storage::manifest::{ManifestHnswProfile, StorageManifest};

#[test]
fn manifest_hnsw_profile_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        hnsw_profile: Some(ManifestHnswProfile {
            max_neighbors: 24,
            ef_search: 192,
            layer_count: 5,
            metric: 1,
        }),
        ..StorageManifest::default()
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}
