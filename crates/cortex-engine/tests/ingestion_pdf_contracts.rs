use cortex_engine::{
    DigitalPdfTextExtractor, DisabledExternalOcrAdapter, ExternalOcrAdapter, ExternalOcrOutput,
    ExternalOcrPageImage, ExternalOcrPageText, ExternalOcrRequest, NativeDigitalPdfTextExtractor,
    PdfTextExtractionBoundary,
};

#[test]
fn native_digital_pdf_extractor_uses_digital_pdf_boundary() {
    let pdf = br#"%PDF-1.4
1 0 obj
<< /Length 48 >>
stream
BT (Digital PDF text) Tj ET
endstream
endobj
%%EOF"#;

    let extractor = NativeDigitalPdfTextExtractor;
    let boundary = PdfTextExtractionBoundary::NativeDigitalPdf;
    let extracted = extractor.extract_text(pdf).unwrap();

    assert_ne!(boundary, PdfTextExtractionBoundary::ExternalOcr);
    assert!(extracted.text.contains("Digital PDF text"));
    assert_eq!(extracted.literal_strings, 1);
}

#[test]
fn external_ocr_request_validation_is_fail_closed() {
    let empty = ExternalOcrRequest {
        document_id: "doc-1",
        source: "scan.pdf",
        pages: Vec::new(),
    };
    assert!(empty.validate().is_err());

    let non_image = ExternalOcrRequest {
        document_id: "doc-1",
        source: "scan.pdf",
        pages: vec![ExternalOcrPageImage {
            page: 1,
            mime_type: "application/pdf",
            bytes: b"not-an-image",
        }],
    };
    assert!(non_image.validate().is_err());
}

#[test]
fn disabled_external_ocr_adapter_requires_explicit_adapter() {
    let request = ExternalOcrRequest {
        document_id: "doc-1",
        source: "scan.pdf",
        pages: vec![ExternalOcrPageImage {
            page: 1,
            mime_type: "image/png",
            bytes: b"png-bytes",
        }],
    };

    let adapter = DisabledExternalOcrAdapter;
    let error = adapter.extract_text(&request).unwrap_err().to_string();

    assert!(error.contains("external OCR adapter is not configured"));
}

#[test]
fn external_ocr_output_combines_pages_in_page_order() {
    let output = ExternalOcrOutput {
        pages: vec![
            ExternalOcrPageText {
                page: 2,
                text: "second page".to_owned(),
                confidence_q16: Some(50_000),
            },
            ExternalOcrPageText {
                page: 1,
                text: "first page".to_owned(),
                confidence_q16: Some(60_000),
            },
        ],
    };

    assert_eq!(output.combined_text(), "first page\n\nsecond page");
}
