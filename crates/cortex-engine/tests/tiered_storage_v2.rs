use cortex_core::CellId;
use cortex_engine::{
    Database, DatabaseOptions, PayloadResidency, TieredStorageCompressionPolicy,
    TieredStorageOptions,
};

#[test]
fn tiered_storage_v2_serves_cold_payloads_with_bounded_hot_cache() {
    let dir = tempfile::tempdir().unwrap();
    let payloads = (1..=5)
        .map(|id| {
            (
                CellId(id),
                format!("scope=x\nstatus=ready\n\ncold-{id}-1234567890").into_bytes(),
            )
        })
        .collect::<Vec<_>>();

    {
        let mut db = Database::open(dir.path()).unwrap();
        for (cell_id, payload) in &payloads {
            db.put_cell(*cell_id, payload.clone()).unwrap();
        }
        db.checkpoint().unwrap();
    }

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            payload_cache_bytes: 64,
            tiered_storage: TieredStorageOptions {
                enabled: true,
                compression_policy: TieredStorageCompressionPolicy::None,
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    assert!(db.tiered_storage_options().enabled);
    assert_eq!(
        db.tiered_storage_options().compression_policy,
        TieredStorageCompressionPolicy::None
    );
    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    assert_eq!(db.payload_cache_stats().max_bytes, 64);

    for (cell_id, payload) in &payloads {
        assert_eq!(db.get_latest_cell(*cell_id).unwrap(), *payload);
        assert!(db.payload_cache_stats().resident_bytes <= 64);
    }

    let stats = db.payload_cache_stats();
    assert_eq!(stats.segment_loads, payloads.len() as u64);
    assert_eq!(stats.misses, payloads.len() as u64);
    assert!(stats.evictions > 0);
    assert!(stats.entries > 0);
    assert!(stats.resident_bytes <= stats.max_bytes);

    let (last_cell_id, last_payload) = payloads.last().unwrap();
    assert_eq!(db.get_latest_cell(*last_cell_id).unwrap(), *last_payload);
    let stats_after_hit = db.payload_cache_stats();
    assert_eq!(stats_after_hit.segment_loads, stats.segment_loads);
    assert_eq!(stats_after_hit.hits, stats.hits + 1);
}
