use cortex_core::CellId;
use cortex_engine::{
    extract_pdf_text, CellMetadata, Database, DisabledExternalPdfParserAdapter,
    ExternalPdfParserAdapter, ExternalPdfParserRequest, PdfIngestOptions,
};

fn two_page_pdf() -> Vec<u8> {
    br#"%PDF-1.4
1 0 obj
<< /Length 33 >>
stream
BT (First page budget) Tj ET
endstream
endobj
2 0 obj
<< /Length 34 >>
stream
BT (Second page timeline) Tj ET
endstream
endobj
%%EOF"#
        .to_vec()
}

#[test]
fn native_pdf_extraction_reports_page_level_text_and_counts() {
    let extracted = extract_pdf_text(&two_page_pdf()).unwrap();

    assert_eq!(extracted.page_count, 2);
    assert_eq!(extracted.literal_strings, 2);
    assert_eq!(extracted.hex_strings, 0);
    assert_eq!(extracted.pages[0].page, 1);
    assert_eq!(extracted.pages[1].page, 2);
    assert!(extracted.pages[0].text.contains("First page budget"));
    assert!(extracted.pages[1].text.contains("Second page timeline"));
    assert!(extracted.text.contains("First page budget"));
    assert!(extracted.text.contains("Second page timeline"));
}

#[test]
fn native_pdf_page_ingestion_writes_page_source_refs_and_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let cells = db
        .ingest_pdf_bytes_pages(
            CellId(500),
            &two_page_pdf(),
            PdfIngestOptions {
                scope: "project:investments".to_owned(),
                source: "report.pdf".to_owned(),
                page: None,
            },
        )
        .unwrap();

    assert_eq!(cells.len(), 2);
    let first = db.get_latest_cell(CellId(500)).unwrap();
    let first_text = String::from_utf8_lossy(&first);
    assert!(first_text.contains("page=1"));
    assert!(first_text.contains("cell_range=page-1"));
    assert!(first_text.contains("source_format=pdf"));
    assert!(first_text.contains("extraction_boundary=native_digital_pdf"));
    assert!(first_text.contains("pdf_page_count=2"));
    assert!(first_text.contains("pdf_page_literal_strings=1"));
    assert!(first_text.contains("First page budget"));

    let first_ref = CellMetadata::from_payload(&first).source_ref.unwrap();
    assert_eq!(first_ref.document_id.as_deref(), Some("report.pdf"));
    assert_eq!(first_ref.page, Some(1));
    assert_eq!(first_ref.cell_range.as_deref(), Some("page-1"));

    let second = db.get_latest_cell(CellId(501)).unwrap();
    let second_ref = CellMetadata::from_payload(&second).source_ref.unwrap();
    assert_eq!(second_ref.page, Some(2));
    assert_eq!(second_ref.cell_range.as_deref(), Some("page-2"));
}

#[test]
fn native_pdf_page_ingestion_can_start_at_known_source_page() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    db.ingest_pdf_bytes_pages(
        CellId(600),
        &two_page_pdf(),
        PdfIngestOptions {
            scope: "project:investments".to_owned(),
            source: "appendix.pdf".to_owned(),
            page: Some(10),
        },
    )
    .unwrap();

    let first = db.get_latest_cell(CellId(600)).unwrap();
    let second = db.get_latest_cell(CellId(601)).unwrap();
    assert_eq!(
        CellMetadata::from_payload(&first).source_ref.unwrap().page,
        Some(10)
    );
    assert_eq!(
        CellMetadata::from_payload(&second).source_ref.unwrap().page,
        Some(11)
    );
}

#[test]
fn pdf_text_ingestion_marks_external_parser_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let result = db
        .ingest_pdf_text(
            CellId(700),
            "Extracted table text",
            PdfIngestOptions {
                scope: "project:investments".to_owned(),
                source: "report.pdf".to_owned(),
                page: Some(7),
            },
        )
        .unwrap();

    let payload = db.get_latest_cell(result.cell_id).unwrap();
    let text = String::from_utf8_lossy(&payload);
    assert!(text.contains("source=report.pdf"));
    assert!(text.contains("source_id=report.pdf"));
    assert!(text.contains("document_id=report.pdf"));
    assert!(text.contains("source_format=pdf"));
    assert!(text.contains("extraction_boundary=external_parser"));
    assert!(text.contains("page=7"));
    let source_ref = CellMetadata::from_payload(&payload).source_ref.unwrap();
    assert_eq!(source_ref.page, Some(7));
}

#[test]
fn native_pdf_ingestion_extracts_simple_text_objects() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let pdf = br#"%PDF-1.4
1 0 obj
<< /Length 64 >>
stream
BT /F1 12 Tf 72 720 Td (ABC budget) Tj <203132303030> Tj ET
endstream
endobj
%%EOF"#;

    let extracted = extract_pdf_text(pdf).unwrap();
    assert_eq!(extracted.literal_strings, 1);
    assert_eq!(extracted.hex_strings, 1);
    assert_eq!(extracted.page_count, 1);
    assert!(extracted.text.contains("ABC budget"));
    assert!(extracted.text.contains("12000"));

    db.ingest_pdf_bytes(
        CellId(800),
        pdf,
        PdfIngestOptions {
            scope: "project:investments".to_owned(),
            source: "native.pdf".to_owned(),
            page: None,
        },
    )
    .unwrap();
    let payload = db.get_latest_cell(CellId(800)).unwrap();
    let text = String::from_utf8_lossy(&payload);
    assert!(text.contains("ABC budget"));
    assert!(text.contains("extraction_boundary=native_digital_pdf"));
    assert!(text.contains("pdf_literal_strings=1"));
    assert!(text.contains("pdf_hex_strings=1"));
}

#[test]
fn native_pdf_ingestion_extracts_flate_decode_streams() {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(b"BT (Compressed budget 12000) Tj ET")
        .unwrap();
    let compressed = encoder.finish().unwrap();
    let mut pdf = b"%PDF-1.4\n1 0 obj\n<< /Filter /FlateDecode >>\nstream\n".to_vec();
    pdf.extend_from_slice(&compressed);
    pdf.extend_from_slice(b"\nendstream\nendobj\n%%EOF");

    let extracted = extract_pdf_text(&pdf).unwrap();

    assert_eq!(extracted.page_count, 1);
    assert!(extracted.text.contains("Compressed budget 12000"));
}

#[test]
fn disabled_external_pdf_parser_adapter_requires_explicit_parser() {
    let request = ExternalPdfParserRequest {
        document_id: "doc-1",
        source: "report.pdf",
        pdf_bytes: &two_page_pdf(),
    };

    let adapter = DisabledExternalPdfParserAdapter;
    let error = adapter.extract_text(&request).unwrap_err().to_string();

    assert!(error.contains("external PDF parser adapter is not configured"));
}

#[test]
fn external_pdf_parser_request_validation_fails_closed() {
    let invalid = ExternalPdfParserRequest {
        document_id: "doc-1",
        source: "report.pdf",
        pdf_bytes: b"not a pdf",
    };

    assert!(invalid.validate().is_err());
}
