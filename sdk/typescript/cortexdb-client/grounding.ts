import type { AnswerGroundingReportResponse, ContextPackResponse, GroundedAnswerResponse, VerificationReportResponse } from "./types";

function uniqueValues<T>(values: T[]): T[] {
  const seen = new Set<T>();
  const out: T[] = [];
  for (const value of values) {
    if (!seen.has(value)) {
      seen.add(value);
      out.push(value);
    }
  }
  return out;
}

function tokenize(text: string): string[] {
  const stopwords = new Set(["a", "an", "and", "the", "or", "of", "to", "in"]);
  const terms = text
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((term) => term.length > 0 && !stopwords.has(term));
  return [...new Set(terms)].sort();
}

function splitAnswerSpans(answer: string): Array<[string, number, number]> {
  const spans: Array<[string, number, number]> = [];
  let start = 0;
  for (let index = 0; index < answer.length; index += 1) {
    const ch = answer[index];
    const decimalDot = ch === "." && /\d/.test(answer[index - 1] ?? "") && /\d/.test(answer[index + 1] ?? "");
    if (ch === "!" || ch === "?" || ch === "\n" || (ch === "." && !decimalDot)) {
      pushAnswerSpan(answer, start, index + 1, spans);
      start = index + 1;
    }
  }
  pushAnswerSpan(answer, start, answer.length, spans);
  return spans;
}

function pushAnswerSpan(
  answer: string,
  start: number,
  end: number,
  spans: Array<[string, number, number]>,
): void {
  const raw = answer.slice(start, end);
  const text = raw.trim();
  if (text.length === 0) return;
  const leading = raw.length - raw.trimStart().length;
  const trailing = raw.length - raw.trimEnd().length;
  spans.push([text, start + leading, end - trailing]);
}

function q16Ratio(numerator: number, denominator: number): number {
  if (denominator === 0) return 65535;
  return Math.floor((numerator * 65535) / denominator);
}

export function groundAnswer(
  context: ContextPackResponse,
  answer: string,
  options: {
    minSpanSupportQ16?: number;
    requireCitations?: boolean;
    rejectUnsupported?: boolean;
  } = {},
): AnswerGroundingReportResponse {
  const minSpanSupportQ16 = options.minSpanSupportQ16 ?? 65535;
  const requireCitations = options.requireCitations ?? false;
  const rejectUnsupported = options.rejectUnsupported ?? false;
  const spans = splitAnswerSpans(answer).map(([text, start, end]) => {
    const spanTerms = tokenize(text);
    if (spanTerms.length === 0) {
      return {
        text,
        start_byte: start,
        end_byte: end,
        support_q16: 65535,
        supported: true,
        covered_terms: [],
        missing_terms: [],
        supported_by_cell_ids: [],
        citations: [],
      };
    }
    const covered = new Set<string>();
    const cellIds: number[] = [];
    const citations: string[] = [];
    for (const cell of context.cells) {
      const cellTerms = new Set(tokenize(cell.payload_text));
      let matched = false;
      for (const term of spanTerms) {
        if (cellTerms.has(term)) {
          covered.add(term);
          matched = true;
        }
      }
      if (matched) {
        cellIds.push(cell.cell_id);
        if (cell.citation) citations.push(cell.citation);
      }
    }
    const support = q16Ratio(covered.size, spanTerms.length);
    const supported = support >= minSpanSupportQ16 && (!requireCitations || citations.length > 0);
    return {
      text,
      start_byte: start,
      end_byte: end,
      support_q16: support,
      supported,
      covered_terms: [...covered].sort(),
      missing_terms: spanTerms.filter((term) => !covered.has(term)),
      supported_by_cell_ids: uniqueValues(cellIds),
      citations: uniqueValues(citations),
    };
  });
  const supportedSpanCount = spans.filter((span) => span.supported).length;
  const unsupportedSpanCount = spans.length - supportedSpanCount;
  const support = spans.length === 0
    ? 65535
    : Math.floor(spans.reduce((total, span) => total + span.support_q16, 0) / spans.length);
  return {
    answer_supported: unsupportedSpanCount === 0,
    rejected: rejectUnsupported && unsupportedSpanCount > 0,
    support_q16: support,
    supported_span_count: supportedSpanCount,
    unsupported_span_count: unsupportedSpanCount,
    spans,
  };
}

export function buildGroundedAnswerResponse(params: {
  question: string;
  answer: string;
  retrieveStatement: string;
  verifyStatement?: string | null;
  context: ContextPackResponse;
  verification?: VerificationReportResponse | null;
  requireCitations?: boolean;
  minSpanSupportQ16?: number;
  rejectUnsupported?: boolean;
}): GroundedAnswerResponse {
  const grounding = groundAnswer(params.context, params.answer, {
    requireCitations: params.requireCitations,
    minSpanSupportQ16: params.minSpanSupportQ16,
    rejectUnsupported: params.rejectUnsupported,
  });
  const citations = uniqueValues([
    ...grounding.spans.flatMap((span) => span.citations),
    ...params.context.cells.flatMap((cell) => cell.citation ? [cell.citation] : []),
  ]);
  const usedContextCellIds = uniqueValues([
    ...grounding.spans.flatMap((span) => span.supported_by_cell_ids),
    ...params.context.cells.map((cell) => cell.cell_id),
  ]);
  return {
    question: params.question,
    answer: params.answer,
    retrieve_statement: params.retrieveStatement,
    verify_statement: params.verifyStatement ?? null,
    context: params.context,
    grounding,
    verification: params.verification ?? null,
    citations,
    used_context_cell_ids: usedContextCellIds,
    rejected: grounding.rejected,
  };
}

