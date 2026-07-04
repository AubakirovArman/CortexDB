//! structured/text chunking unit tests (moved from chunking.rs; behavior unchanged).

use super::*;

fn structured(text: &str) -> Vec<StructuredChunk> {
    split_text_chunks_structured("doc", text, TextChunkPolicy::default()).unwrap()
}

fn table(rows_per_group: usize) -> Vec<StructuredChunk> {
    let header = vec!["item".to_owned(), "price".to_owned()];
    let rows = vec![
        vec!["apple".to_owned(), "3".to_owned()],
        vec!["pear".to_owned(), "5".to_owned()],
        vec!["plum".to_owned(), "7".to_owned()],
    ];
    split_table_rows_structured(
        "t",
        &header,
        &rows,
        rows_per_group,
        TableChunkPolicy::default(),
    )
    .unwrap()
}

#[test]
fn table_parent_lists_columns_and_rows_are_key_value_tokenized() {
    let chunks = table(1);
    assert_eq!(chunks[0].role, StructuredChunkRole::Parent);
    assert!(chunks[0].text.contains("item") && chunks[0].text.contains("price"));
    // A "column + value" query token ("price: 5") lands in the pear row-group.
    let pear = chunks
        .iter()
        .find(|c| c.text.contains("pear"))
        .expect("pear row");
    assert!(pear.text.contains("item: pear") && pear.text.contains("price: 5"));
    assert_eq!(pear.parent_id.as_deref(), Some(chunks[0].chunk_id.as_str()));
    assert_eq!(pear.breadcrumb, "item | price");
}

#[test]
fn row_grouping_and_source_rows_are_respected() {
    // 3 rows grouped 2-per-group -> parent + 2 children (2 rows, then 1 row).
    let chunks = table(2);
    assert_eq!(chunks.len(), 3);
    assert!(chunks[1].text.contains("apple") && chunks[1].text.contains("pear"));
    assert!(chunks[2].text.contains("plum"));
    // First data row provenance (TableChunkPolicy default first_data_row=2).
    assert!(chunks[1].text.contains("row-2:"));
}

#[test]
fn table_chunking_is_deterministic() {
    assert_eq!(table(2), table(2));
}

#[test]
fn parent_summary_is_the_heading_outline_and_children_carry_breadcrumbs() {
    let doc = "# Runbook\n\nintro text\n\n## Recovery\n\nrestore from backup\n\n### Steps\n\ndo the thing";
    let chunks = structured(doc);
    assert_eq!(chunks[0].role, StructuredChunkRole::Parent);
    assert_eq!(chunks[0].parent_id, None);
    assert!(chunks[0].text.contains("Runbook > Recovery > Steps"));
    // Every child points at the parent and carries its heading breadcrumb.
    let steps = chunks
        .iter()
        .find(|c| c.text.contains("do the thing"))
        .expect("steps chunk");
    assert_eq!(steps.role, StructuredChunkRole::Child);
    assert_eq!(
        steps.parent_id.as_deref(),
        Some(chunks[0].chunk_id.as_str())
    );
    assert_eq!(steps.breadcrumb, "Runbook > Recovery > Steps");
}

#[test]
fn a_fenced_code_block_is_never_split_and_hashes_inside_are_not_headings() {
    let doc = "# Code\n\n```\nfn main() {\n    # not a heading\n\n    let x = 1;\n}\n```\n\nafter";
    let chunks = structured(doc);
    let code = chunks
        .iter()
        .find(|c| c.text.contains("fn main()"))
        .expect("code chunk");
    // The whole fence (including its blank line and the '#' line) stays intact.
    assert!(code.text.contains("# not a heading"));
    assert!(code.text.contains("let x = 1;"));
    assert_eq!(code.breadcrumb, "Code");
}

#[test]
fn a_run_of_table_rows_stays_one_chunk() {
    let doc = "## Prices\n\n| item | price |\n| --- | --- |\n| a | 1 |\n| b | 2 |";
    let chunks = structured(doc);
    let table = chunks
        .iter()
        .find(|c| c.text.contains("| item | price |"))
        .expect("table chunk");
    assert!(table.text.contains("| a | 1 |") && table.text.contains("| b | 2 |"));
}

#[test]
fn deterministic_same_bytes_same_chunks() {
    let doc = "# A\n\nalpha\n\n## B\n\nbeta";
    assert_eq!(structured(doc), structured(doc));
}

#[test]
fn heading_less_document_produces_a_lead_summary_and_children() {
    let doc = "just some prose\n\nmore prose";
    let chunks = structured(doc);
    assert_eq!(chunks[0].role, StructuredChunkRole::Parent);
    assert!(chunks[0].text.contains("just some prose"));
    assert!(chunks
        .iter()
        .skip(1)
        .all(|c| c.role == StructuredChunkRole::Child));
}
