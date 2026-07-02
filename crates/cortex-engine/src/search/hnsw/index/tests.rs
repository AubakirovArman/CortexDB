// A3.1 graph-descent HNSW build: recall vs the exact oracle, byte-identical
// double build (determinism), degree bounds, and integrity after build+remove.
// Deterministic — no RNG or wall-clock assertions. Cell ids start at 1 (0 is a
// reserved sentinel the integrity check rejects).

use super::*;
use crate::search::DistanceMetric;

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

const CLUSTERS: u64 = 24;

fn cluster_center(cluster: u64, dimension: usize) -> Vec<i32> {
    let mut state = cluster.wrapping_mul(0x0123_4567_89ab_cdef).wrapping_add(1);
    (0..dimension)
        .map(|_| (splitmix(&mut state) % 1_601) as i32 - 800)
        .collect()
}

/// Deterministic clustered i16 vector: `seed` picks a cluster center plus small
/// per-vector noise, so near-duplicates cluster the way real embeddings do.
fn vector_for(seed: u64, dimension: usize) -> Vec<i16> {
    let center = cluster_center(seed % CLUSTERS, dimension);
    let mut noise = seed.wrapping_add(0x00ab_cdef);
    center
        .iter()
        .map(|coord| {
            let jitter = (splitmix(&mut noise) % 121) as i32 - 60;
            (coord + jitter).clamp(-1_000, 1_000) as i16
        })
        .collect()
}

fn build_config(count: u32, dimension: usize, max_neighbors: usize, ef_search: usize) -> HnswIndex {
    let mut index = HnswIndex::new_multilayer(max_neighbors, ef_search, 4);
    index.set_config(VectorCollectionConfig {
        dimension,
        metric: DistanceMetric::DotProduct,
    });
    for cell_id in 1..=count {
        index
            .add_vector(cell_id, vector_for(u64::from(cell_id), dimension))
            .unwrap();
    }
    index
}

fn build_index(count: u32, dimension: usize) -> HnswIndex {
    build_config(count, dimension, 8, 48)
}

#[test]
fn graph_descent_recall_matches_exact_within_tolerance() {
    let dimension = 24;
    let count = 400;
    let index = build_config(count, dimension, 16, 160);

    let mut total_recall = 0.0;
    let queries: u32 = 40;
    for q in 0..queries {
        let query = vector_for(1_000_000 + u64::from(q), dimension);
        let exact: std::collections::BTreeSet<u32> =
            index.nearest_existing(&query, 10).into_iter().collect();
        let ann: std::collections::BTreeSet<u32> = index
            .search(&query, 10)
            .into_iter()
            .map(|candidate| candidate.cell_id)
            .collect();
        let hit = exact.iter().filter(|id| ann.contains(id)).count();
        total_recall += hit as f64 / exact.len().max(1) as f64;
    }
    let recall = total_recall / f64::from(queries);
    assert!(
        recall >= 0.95,
        "graph-descent recall@10 too low: {recall:.3}"
    );
}

#[test]
fn double_build_is_byte_identical() {
    let a = build_index(300, 16).graph_index();
    let b = build_index(300, 16).graph_index();
    assert_eq!(a, b, "rebuild must be byte-identical (deterministic)");
}

#[test]
fn build_bounds_degree_and_registers_all_nodes() {
    let count = 1_000;
    let index = build_index(count, 16);
    assert_eq!(index.vector_count(), count as usize);
    let graph = index.graph_index();
    // Layer-0 degree is bounded (no O(N) fan-out from the old full-scan insert).
    // M0 = 2*M = 16; allow a small slack for the last-inserted-before-prune node.
    let max_degree = graph.links.values().map(|n| n.len()).max().unwrap_or(0);
    assert!(
        max_degree <= 16 + 1,
        "layer-0 degree exceeds the M0 bound: {max_degree}"
    );
    assert_eq!(graph.links.len(), count as usize);
}

#[test]
fn integrity_holds_after_build_and_remove() {
    let mut index = build_index(250, 16);
    assert!(
        index.verify_hnsw_integrity(),
        "graph integrity must hold after build: {}",
        index.integrity_report().summary()
    );
    for cell_id in (7..=250).step_by(7) {
        index.remove_vector(cell_id);
    }
    assert!(
        index.verify_hnsw_integrity(),
        "graph integrity must hold after removals: {}",
        index.integrity_report().summary()
    );
    let query = vector_for(42, 16);
    for candidate in index.search(&query, 10) {
        assert!(
            candidate.cell_id % 7 != 0,
            "search returned removed node {}",
            candidate.cell_id
        );
    }
}
