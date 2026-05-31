use cortex_storage::manifest::{ManifestHnswProfile, ManifestVectorProfile, StorageManifest};

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
            ef_construction: 256,
        }),
        ..StorageManifest::default()
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}

#[test]
fn manifest_vector_profile_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        vector_profile: Some(ManifestVectorProfile {
            dimension: 384,
            metric: 1,
        }),
        ..StorageManifest::default()
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}

#[test]
fn manifest_rejects_zero_vector_dimension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        vector_profile: Some(ManifestVectorProfile {
            dimension: 1,
            metric: 0,
        }),
        ..StorageManifest::default()
    };
    manifest.store(&path).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let body_len = bytes.len() - 4;
    let vector_magic = bytes
        .windows(4)
        .position(|window| window == b"VECM")
        .unwrap();
    bytes[vector_magic + 4..vector_magic + 8].copy_from_slice(&0u32.to_le_bytes());
    bytes.truncate(body_len);
    let checksum = crc32c::crc32c(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();

    assert!(StorageManifest::load(&path).is_err());
}
