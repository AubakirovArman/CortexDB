use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::adapters::{IngestedCell, PdfIngestOptions};
use crate::ingestion::cells::{
    document_metadata, offset_cell_id, put_source_ref_cell, SourceRefHeaders,
};
use crate::ingestion::pdf::{extract_pdf_text, PdfExtractedPageText, PdfExtractionStats};

impl Database {
    pub fn ingest_pdf_text(
        &mut self,
        cell_id: CellId,
        extracted_text: &str,
        options: PdfIngestOptions,
    ) -> EngineResult<IngestedCell> {
        put_pdf_cell(
            self,
            cell_id,
            extracted_text,
            &options,
            options.page,
            None,
            &[
                ("source_format", "pdf".to_owned()),
                ("extraction_boundary", "external_parser".to_owned()),
            ],
        )
    }

    pub fn ingest_pdf_bytes(
        &mut self,
        cell_id: CellId,
        pdf: &[u8],
        options: PdfIngestOptions,
    ) -> EngineResult<IngestedCell> {
        let extracted = extract_pdf_text(pdf)?;
        let headers = extraction_headers(&extracted);
        put_pdf_cell(
            self,
            cell_id,
            &extracted.text,
            &options,
            options.page,
            None,
            &headers,
        )
    }

    pub fn ingest_pdf_bytes_pages(
        &mut self,
        first_cell_id: CellId,
        pdf: &[u8],
        options: PdfIngestOptions,
    ) -> EngineResult<Vec<IngestedCell>> {
        let extracted = extract_pdf_text(pdf)?;
        if extracted.pages.is_empty() {
            return Err(EngineError::InvalidOperation);
        }
        extracted
            .pages
            .iter()
            .enumerate()
            .map(|(index, page)| {
                let cell_id = offset_cell_id(first_cell_id, index)?;
                let source_page = source_page_number(options.page, page)?;
                let cell_range = format!("page-{source_page}");
                let headers = page_extraction_headers(&extracted, page);
                put_pdf_cell(
                    self,
                    cell_id,
                    &page.text,
                    &options,
                    Some(source_page),
                    Some(&cell_range),
                    &headers,
                )
            })
            .collect()
    }
}

fn put_pdf_cell(
    db: &mut Database,
    cell_id: CellId,
    text: &str,
    options: &PdfIngestOptions,
    page: Option<u32>,
    cell_range: Option<&str>,
    extra_headers: &[(&str, String)],
) -> EngineResult<IngestedCell> {
    let source = options.source.clone();
    let commit_seq = put_source_ref_cell(
        db,
        cell_id,
        document_metadata(options.scope.clone(), source.clone()),
        text,
        SourceRefHeaders {
            document_id: &source,
            page,
            row: None,
            cell_range,
            json_path: None,
            confidence_q16: None,
            extra_headers,
        },
    )?;
    Ok(IngestedCell {
        cell_id,
        commit_seq,
        chunk_id: None,
    })
}

fn source_page_number(base_page: Option<u32>, page: &PdfExtractedPageText) -> EngineResult<u32> {
    let offset = page
        .page
        .checked_sub(1)
        .ok_or(EngineError::InvalidOperation)?;
    base_page
        .unwrap_or(1)
        .checked_add(offset)
        .ok_or_else(|| EngineError::StorageInvariant("PDF page number overflow".to_owned()))
}

fn extraction_headers(extracted: &PdfExtractionStats) -> Vec<(&'static str, String)> {
    vec![
        ("source_format", "pdf".to_owned()),
        ("extraction_boundary", "native_digital_pdf".to_owned()),
        ("pdf_page_count", extracted.page_count.to_string()),
        ("pdf_literal_strings", extracted.literal_strings.to_string()),
        ("pdf_hex_strings", extracted.hex_strings.to_string()),
    ]
}

fn page_extraction_headers(
    extracted: &PdfExtractionStats,
    page: &PdfExtractedPageText,
) -> Vec<(&'static str, String)> {
    vec![
        ("source_format", "pdf".to_owned()),
        ("extraction_boundary", "native_digital_pdf".to_owned()),
        ("pdf_page_count", extracted.page_count.to_string()),
        ("pdf_page_literal_strings", page.literal_strings.to_string()),
        ("pdf_page_hex_strings", page.hex_strings.to_string()),
    ]
}
