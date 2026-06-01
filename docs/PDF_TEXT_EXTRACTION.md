# PDF Text Extraction Boundary

CortexDB Core Beta keeps PDF text extraction deliberately split into two
contracts:

1. **Native digital PDF extraction** for simple text-bearing PDF files.
2. **External OCR adapter contract** for scanned pages and image-based PDFs.

The database does not claim production OCR. OCR must be supplied by an explicit
external adapter.

## Native Digital PDF

`NativeDigitalPdfTextExtractor` implements `DigitalPdfTextExtractor` and calls
the local `extract_pdf_text` helper. This path is intended only for PDFs that
already contain extractable text objects:

- literal strings inside `BT ... ET`;
- hex strings inside `BT ... ET`;
- simple `/FlateDecode` zlib streams.

Unsupported or empty PDFs fail closed with an engine error instead of storing an
empty cell.

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

## Non-goals

- PDF layout reconstruction.
- Table/form extraction.
- Page image rendering.
- Built-in OCR model execution.
- Legal-grade document interpretation.
