import type { AqlRetrievalMode, RetrieveContextAqlOptions } from "./types";

function quoteAqlString(value: string): string {
  return `"${value
    .replaceAll("\\", "\\\\")
    .replaceAll("\"", "\\\"")
    .replaceAll("\n", "\\n")
    .replaceAll("\r", "\\r")
    .replaceAll("\t", "\\t")}"`;
}

function validateAqlIdentifier(field: string, value: string): void {
  if (!/^[A-Za-z_][A-Za-z0-9_:-]*$/.test(value)) {
    throw new Error(`${field} must be an AQL identifier`);
  }
}

function validateDecimal(field: string, value: string | undefined): void {
  if (value !== undefined && !/^[0-9]+\.[0-9]+$/.test(value)) {
    throw new Error(`${field} must be a decimal literal`);
  }
}

function validateAqlMode(value: AqlRetrievalMode | undefined): void {
  if (
    value !== undefined &&
    value !== "fast" &&
    value !== "balanced" &&
    value !== "semantic" &&
    value !== "audit"
  ) {
    throw new Error("mode must be fast, balanced, semantic, or audit");
  }
}

export function buildRetrieveContextAql(
  task: string,
  brain: string,
  options: RetrieveContextAqlOptions = {},
): string {
  validateAqlIdentifier("brain", brain);
  validateAqlMode(options.mode);
  if (options.whereClause !== undefined && options.whereClause.trim().length === 0) {
    throw new Error("whereClause must not be empty");
  }
  validateDecimal("minConfidence", options.minConfidence);
  validateDecimal("sourceTrust", options.sourceTrust);

  const parts: string[] = [];
  if (options.explain) parts.push("EXPLAIN");
  parts.push("RETRIEVE CONTEXT FOR TASK", quoteAqlString(task), "IN BRAIN", brain);
  if (options.mode) parts.push("USING MODE", options.mode);
  if (options.budgetTokens !== undefined) {
    parts.push("BUDGET", String(options.budgetTokens), "TOKENS");
  }
  if (options.limitCandidates !== undefined) {
    parts.push("LIMIT", String(options.limitCandidates), "CANDIDATES");
  }
  if (options.whereClause !== undefined) {
    parts.push("WHERE", options.whereClause.trim());
  }
  if (options.requireCitations) parts.push("REQUIRE", "citations");
  if (options.minConfidence !== undefined) {
    parts.push("REQUIRE", "confidence", ">=", options.minConfidence);
  }
  if (options.sourceTrust !== undefined) {
    parts.push("REQUIRE", "source_trust", ">=", options.sourceTrust);
  }
  if (options.freshnessSeconds !== undefined) {
    parts.push("REQUIRE", "freshness", "<=", String(options.freshnessSeconds), "SECONDS");
  }
  return `${parts.join(" ")};`;
}

export function buildVerifyFactAql(fact: string, brain: string): string {
  validateAqlIdentifier("brain", brain);
  return `VERIFY FACT ${quoteAqlString(fact)} IN BRAIN ${brain};`;
}

export function buildRememberAql(
  content: string,
  scope: string,
  memoryType: string,
  ttlSeconds?: number,
): string {
  validateAqlIdentifier("scope", scope);
  validateAqlIdentifier("memoryType", memoryType);
  let statement = `REMEMBER ${quoteAqlString(content)} IN SCOPE ${scope} AS TYPE ${memoryType}`;
  if (ttlSeconds !== undefined) statement += ` TTL ${ttlSeconds} SECONDS`;
  return `${statement};`;
}

