use cortex_core::CellId;
use cortex_engine::{Database, DatabaseOptions, DistanceMetric, HnswBuildConfig, HnswBuildProfile};
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::manifest::StorageManifest;

#[test]
fn checkpoint_manifest_persists_intended_hnsw_profile() {
    let dir = tempfile::tempdir().unwrap();
    let config = HnswBuildConfig::for_profile(HnswBuildProfile::Semantic);
    let mut db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            hnsw_build_config: config,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    put_vector_cell(&mut db);
    db.checkpoint().unwrap();
    drop(db);

    let manifest = StorageManifest::load(dir.path().join("manifest.acm")).unwrap();
    let profile = manifest.hnsw_profile.unwrap();

    assert_eq!(profile.max_neighbors, config.max_neighbors as u32);
    assert_eq!(profile.ef_search, config.ef_search as u32);
    assert_eq!(profile.layer_count, config.layer_count as u32);
    assert_eq!(profile.metric, config.metric as u32);
}

#[test]
fn checkpoint_manifest_persists_vector_collection_profile() {
    let dir = tempfile::tempdir().unwrap();
    let config = HnswBuildConfig {
        metric: DistanceMetric::Cosine,
        ..HnswBuildConfig::for_profile(HnswBuildProfile::Balanced)
    };
    let mut db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            hnsw_build_config: config,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    put_vector_cell(&mut db);
    db.checkpoint().unwrap();
    drop(db);

    let manifest = StorageManifest::load(dir.path().join("manifest.acm")).unwrap();
    let profile = manifest.vector_profile.unwrap();

    assert_eq!(profile.dimension, 2);
    assert_eq!(profile.metric, DistanceMetric::Cosine as u32);
}

#[test]
fn validation_rejects_hnsw_graph_profile_that_differs_from_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            hnsw_build_config: HnswBuildConfig::for_profile(HnswBuildProfile::Semantic),
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    put_vector_cell(&mut db);
    db.checkpoint().unwrap();
    drop(db);

    HnswGraphIndex {
        max_neighbors: 8,
        ef_search: 64,
        layer_count: 3,
        metric: DistanceMetric::Cosine as u8,
        ..HnswGraphIndex::default()
    }
    .write(dir.path().join("segments/segment-1.ach"))
    .unwrap();

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();

    assert!(error.contains("does not match manifest profile"));
    assert!(error.contains("max_neighbors=8"));
    assert!(error.contains("max_neighbors=24"));
}

#[test]
fn validation_rejects_vector_profile_that_differs_from_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    put_vector_cell(&mut db);
    db.checkpoint().unwrap();
    drop(db);

    let manifest_path = dir.path().join("manifest.acm");
    let mut manifest = StorageManifest::load(&manifest_path).unwrap();
    manifest.vector_profile.as_mut().unwrap().dimension = 3;
    manifest.store(&manifest_path).unwrap();

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();

    assert!(error.contains("vector collection"));
    assert!(error.contains("dimension=2"));
    assert!(error.contains("dimension=3"));
}

#[test]
fn validation_rejects_hnsw_dimension_that_differs_from_vector_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    put_vector_cell(&mut db);
    db.checkpoint().unwrap();
    drop(db);

    let graph_path = dir.path().join("segments/segment-1.ach");
    let mut graph = HnswGraphIndex::read(&graph_path).unwrap();
    graph.dimension = 3;
    graph.write(&graph_path).unwrap();

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();

    assert!(error.contains("dimension 3 does not match vector index dimension 2"));
}

#[test]
fn checkpoint_rejects_new_vector_dimension_that_conflicts_with_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    put_vector_cell(&mut db);
    db.checkpoint().unwrap();
    drop(db);

    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nvector=10,0,1\n\nbeta".to_vec(),
    )
    .unwrap();

    let error = db.checkpoint().unwrap_err().to_string();

    assert!(error.contains("checkpoint vector profile"));
    assert!(error.contains("dimension: 3"));
    assert!(error.contains("dimension: 2"));
}

fn put_vector_cell(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=10,0\n\nalpha".to_vec(),
    )
    .unwrap();
}
