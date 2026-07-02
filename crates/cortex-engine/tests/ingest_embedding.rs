//! A2.2: text ingest can auto-embed cell bodies through an injected `Embedder`,
//! writing a `vector=` header so cells become retrievable by vector search. The
//! engine stays network-free; these tests use the offline `DeterministicTestEmbedder`.

use cortex_core::CellId;
use cortex_engine::{
    Database, DeterministicTestEmbedder, IngestionUpdatePolicy, TextChunkPolicy, TextIngestOptions,
};

fn opts(scope: &str, source: &str) -> TextIngestOptions {
    TextIngestOptions {
        scope: scope.to_owned(),
        source: source.to_owned(),
    }
}

#[test]
fn ingest_with_embedder_writes_vector_headers() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let embedder = DeterministicTestEmbedder::new(8);

    let ingested = db
        .ingest_text_chunks_with_embedder(
            CellId(1_000),
            "solar plant capital budget approved for 2025",
            opts("project:embed", "doc-1"),
            TextChunkPolicy::default(),
            IngestionUpdatePolicy::AlwaysInsert,
            &embedder,
        )
        .unwrap();

    assert!(!ingested.is_empty());
    for cell in &ingested {
        let payload = db.get_latest_cell(cell.cell_id).unwrap();
        let text = String::from_utf8_lossy(&payload);
        let line = text
            .lines()
            .find(|line| line.starts_with("vector="))
            .expect("vector header missing");
        // 8 comma-separated i16 lanes, matching the embedder dimension.
        assert_eq!(line["vector=".len()..].split(',').count(), 8);
    }
}

#[test]
fn embedded_ingest_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let embedder = DeterministicTestEmbedder::new(8);
    let text = "solar plant capital budget approved for 2025";

    let first = db
        .ingest_text_chunks_with_embedder(
            CellId(1_000),
            text,
            opts("project:embed", "doc-a"),
            TextChunkPolicy::default(),
            IngestionUpdatePolicy::AlwaysInsert,
            &embedder,
        )
        .unwrap();
    let second = db
        .ingest_text_chunks_with_embedder(
            CellId(2_000),
            text,
            opts("project:embed", "doc-a"),
            TextChunkPolicy::default(),
            IngestionUpdatePolicy::AlwaysInsert,
            &embedder,
        )
        .unwrap();

    let vector_of = |db: &Database, cell_id: CellId| -> String {
        let payload = db.get_latest_cell(cell_id).unwrap();
        String::from_utf8_lossy(&payload)
            .lines()
            .find(|line| line.starts_with("vector="))
            .unwrap()
            .to_owned()
    };
    assert_eq!(first.len(), second.len());
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(vector_of(&db, a.cell_id), vector_of(&db, b.cell_id));
    }
}

#[test]
fn ingest_without_embedder_writes_no_vector_header() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let ingested = db
        .ingest_text_chunks(
            CellId(1_000),
            "wind farm operating budget rejected",
            opts("project:embed", "doc-2"),
        )
        .unwrap();

    assert!(!ingested.is_empty());
    for cell in &ingested {
        let payload = db.get_latest_cell(cell.cell_id).unwrap();
        assert!(!String::from_utf8_lossy(&payload).contains("vector="));
    }
}
