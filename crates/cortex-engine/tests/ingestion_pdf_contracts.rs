use cortex_engine::{
    DigitalPdfTextExtractor, DisabledExternalOcrAdapter, ExternalOcrAdapter,
    ExternalOcrBoundingBox, ExternalOcrOutput, ExternalOcrPageImage, ExternalOcrPageText,
    ExternalOcrRequest, ExternalOcrTextBlock, NativeDigitalPdfTextExtractor,
    PdfTextExtractionBoundary, ScannedPdfOcrRequest,
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
                blocks: Vec::new(),
            },
            ExternalOcrPageText {
                page: 1,
                text: "first page".to_owned(),
                confidence_q16: Some(60_000),
                blocks: Vec::new(),
            },
        ],
    };

    assert_eq!(output.combined_text(), "first page\n\nsecond page");
}

#[test]
fn scanned_pdf_ocr_boundary_converts_to_external_ocr_request() {
    let scanned = ScannedPdfOcrRequest {
        document_id: "doc-1",
        source: "scan.pdf",
        rendered_pages: vec![ExternalOcrPageImage {
            page: 3,
            mime_type: "image/png",
            bytes: b"png-bytes",
        }],
    };

    scanned.validate().unwrap();
    let request = scanned.into_ocr_request();

    assert_eq!(request.document_id, "doc-1");
    assert_eq!(request.source, "scan.pdf");
    assert_eq!(request.pages[0].page, 3);
}

#[test]
fn external_ocr_output_validates_confidence_and_bbox_metadata() {
    let output = ExternalOcrOutput {
        pages: vec![ExternalOcrPageText {
            page: 1,
            text: "page text".to_owned(),
            confidence_q16: Some(61_000),
            blocks: vec![ExternalOcrTextBlock {
                text: "line text".to_owned(),
                confidence_q16: Some(60_000),
                bbox: Some(ExternalOcrBoundingBox {
                    x_q16: 1_000,
                    y_q16: 2_000,
                    width_q16: 10_000,
                    height_q16: 4_000,
                }),
            }],
        }],
    };

    output.validate().unwrap();
}

#[test]
fn external_ocr_output_rejects_invalid_bbox_metadata() {
    let output = ExternalOcrOutput {
        pages: vec![ExternalOcrPageText {
            page: 1,
            text: "page text".to_owned(),
            confidence_q16: Some(61_000),
            blocks: vec![ExternalOcrTextBlock {
                text: "line text".to_owned(),
                confidence_q16: Some(60_000),
                bbox: Some(ExternalOcrBoundingBox {
                    x_q16: 65_000,
                    y_q16: 0,
                    width_q16: 1_000,
                    height_q16: 1_000,
                }),
            }],
        }],
    };

    assert!(output.validate().is_err());
}
