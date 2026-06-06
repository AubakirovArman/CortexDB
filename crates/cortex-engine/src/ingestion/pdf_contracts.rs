use crate::error::{EngineError, EngineResult};

use super::pdf::{extract_pdf_text, PdfExtractionStats};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfTextExtractionBoundary {
    NativeDigitalPdf,
    ExternalParser,
    ExternalOcr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalPdfParserRequest<'a> {
    pub document_id: &'a str,
    pub source: &'a str,
    pub pdf_bytes: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalOcrPageImage<'a> {
    pub page: u32,
    pub mime_type: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalOcrRequest<'a> {
    pub document_id: &'a str,
    pub source: &'a str,
    pub pages: Vec<ExternalOcrPageImage<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalOcrPageText {
    pub page: u32,
    pub text: String,
    pub confidence_q16: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalOcrOutput {
    pub pages: Vec<ExternalOcrPageText>,
}

pub trait DigitalPdfTextExtractor {
    fn extract_text(&self, pdf_bytes: &[u8]) -> EngineResult<PdfExtractionStats>;
}

pub trait ExternalPdfParserAdapter {
    fn extract_text(
        &self,
        request: &ExternalPdfParserRequest<'_>,
    ) -> EngineResult<PdfExtractionStats>;
}

pub trait ExternalOcrAdapter {
    fn extract_text(&self, request: &ExternalOcrRequest<'_>) -> EngineResult<ExternalOcrOutput>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeDigitalPdfTextExtractor;

#[derive(Clone, Copy, Debug, Default)]
pub struct DisabledExternalPdfParserAdapter;

#[derive(Clone, Copy, Debug, Default)]
pub struct DisabledExternalOcrAdapter;

impl DigitalPdfTextExtractor for NativeDigitalPdfTextExtractor {
    fn extract_text(&self, pdf_bytes: &[u8]) -> EngineResult<PdfExtractionStats> {
        extract_pdf_text(pdf_bytes)
    }
}

impl ExternalPdfParserAdapter for DisabledExternalPdfParserAdapter {
    fn extract_text(
        &self,
        request: &ExternalPdfParserRequest<'_>,
    ) -> EngineResult<PdfExtractionStats> {
        request.validate()?;
        Err(EngineError::StorageInvariant(
            "external PDF parser adapter is not configured".to_owned(),
        ))
    }
}

impl ExternalOcrAdapter for DisabledExternalOcrAdapter {
    fn extract_text(&self, request: &ExternalOcrRequest<'_>) -> EngineResult<ExternalOcrOutput> {
        validate_external_ocr_request(request)?;
        Err(EngineError::StorageInvariant(
            "external OCR adapter is not configured".to_owned(),
        ))
    }
}

impl<'a> ExternalPdfParserRequest<'a> {
    pub fn validate(&self) -> EngineResult<()> {
        if self.document_id.trim().is_empty()
            || self.source.trim().is_empty()
            || !self.pdf_bytes.starts_with(b"%PDF-")
        {
            return Err(EngineError::InvalidOperation);
        }
        Ok(())
    }
}

impl<'a> ExternalOcrRequest<'a> {
    pub fn validate(&self) -> EngineResult<()> {
        validate_external_ocr_request(self)
    }
}

impl ExternalOcrOutput {
    pub fn combined_text(&self) -> String {
        let mut pages = self.pages.clone();
        pages.sort_by_key(|page| page.page);
        pages
            .into_iter()
            .map(|page| page.text.trim().to_owned())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

pub fn validate_external_ocr_request(request: &ExternalOcrRequest<'_>) -> EngineResult<()> {
    if request.document_id.trim().is_empty()
        || request.source.trim().is_empty()
        || request.pages.is_empty()
    {
        return Err(EngineError::InvalidOperation);
    }
    for page in &request.pages {
        if page.page == 0 || page.bytes.is_empty() || !page.mime_type.starts_with("image/") {
            return Err(EngineError::InvalidOperation);
        }
    }
    Ok(())
}
