export type AqlRetrievalMode = "fast" | "balanced" | "semantic" | "audit";

export interface AqlCellResponse {
  cell_id: number;
  payload: string;
}

export interface AqlResponse {
  cells: AqlCellResponse[];
}

export interface RetrieveContextAqlOptions {
  mode?: AqlRetrievalMode;
  budgetTokens?: number;
  limitCandidates?: number;
  whereClause?: string;
  requireCitations?: boolean;
  minConfidence?: string;
  sourceTrust?: string;
  freshnessSeconds?: number;
  explain?: boolean;
}
