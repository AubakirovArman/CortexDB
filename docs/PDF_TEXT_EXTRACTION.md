# PDF Text Extraction Boundary

CortexDB Core Beta keeps PDF text extraction deliberately split into three
contracts:

1. **Native digital PDF extraction** for simple text-bearing PDF files.
2. **External PDF parser adapter contract** for production-grade digital PDF
   parsing outside the embedded core.
3. **External OCR adapter contract** for scanned pages and image-based PDFs.

The database does not claim production PDF layout parsing or production OCR.
Those paths must be supplied by explicit external adapters.

## Native Digital PDF

`NativeDigitalPdfTextExtractor` implements `DigitalPdfTextExtractor` and calls
the local `extract_pdf_text` helper. This path is intended only for PDFs that
already contain extractable text objects:

- literal strings inside `BT ... ET`;
- hex strings inside `BT ... ET`;
- simple `/FlateDecode` zlib streams.

Unsupported or empty PDFs fail closed with an engine error instead of storing an
empty cell.

`PdfExtractionStats` records:

- combined extracted text;
- `page_count`;
- page-level text blocks in `pages`;
- literal-string and hex-string counts.

`Database::ingest_pdf_bytes_pages` writes one document-block cell per extracted
page. Each emitted cell carries structured SourceRef metadata:

```text
document_id=<source>
page=<source page number>
cell_range=page-<source page number>
source_format=pdf
extraction_boundary=native_digital_pdf
pdf_page_count=<n>
```

The native path assigns page numbers by text-bearing content-stream order. If
the caller knows that the PDF fragment starts at a later source page, pass
`PdfIngestOptions.page`; emitted SourceRefs will start at that page and advance
by one per extracted page.

## External PDF Parser

`ExternalPdfParserAdapter` is the extension point for a real PDF parser such as
a layout-aware service or a process that uses a dedicated PDF parsing library.
The request boundary is `ExternalPdfParserRequest`:

- `document_id`;
- `source`;
- raw `pdf_bytes`.

`DisabledExternalPdfParserAdapter` validates that the request is a PDF-shaped
input and then fails closed with `external PDF parser adapter is not configured`.
This keeps the local core honest: the native adapter is a small deterministic
digital extractor, not the production parsing path for complicated PDFs.

## External OCR

`ExternalOcrAdapter` is the only OCR extension point. It accepts an
`ExternalOcrRequest` with:

- `document_id`;
- `source`;
- one or more page images;
- image MIME type;
- non-empty image bytes.

`DisabledExternalOcrAdapter` validates the request and then fails with
`external OCR adapter is not configured`. This makes the alpha/beta boundary
explicit: CortexDB can ingest OCR text through a configured adapter later, but
it does not silently pretend that scanned PDFs are supported today.

## Output Contract

External OCR returns `ExternalOcrOutput` with page-level text and optional
`confidence_q16`. `combined_text()` sorts pages by page number and joins
non-empty page text with blank lines, producing a deterministic text body that
can then flow through normal text chunking and SourceRef metadata.

## Verification

Run the focused gate:

```bash
make pdf-digital-adapter-check
```

## Non-goals

- PDF layout reconstruction.
- Table/form extraction.
- Page image rendering.
- Built-in OCR model execution.
- Legal-grade document interpretation.
