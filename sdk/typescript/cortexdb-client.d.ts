export type JsonObject = Record<string, unknown>;
export type VectorAlgorithm = "ann" | "exact";

export class CortexDBClient {
  constructor(baseUrl?: string, token?: string);
  health(): Promise<JsonObject>;
  putCell(cellId: number, payload: string): Promise<JsonObject>;
  getCell(cellId: number): Promise<JsonObject>;
  tombstoneCell(cellId: number): Promise<JsonObject>;
  flush(): Promise<JsonObject>;
  compact(): Promise<JsonObject>;
  search(scope: string, query: string, limit?: number): Promise<JsonObject>;
  searchVector(
    scope: string,
    vector: number[],
    limit?: number,
    algorithm?: VectorAlgorithm,
  ): Promise<JsonObject>;
  aql(scope: string, statement: string): Promise<JsonObject>;
  retrieveContext(scope: string, statement: string): Promise<JsonObject>;
  verifyFact(scope: string, statement: string): Promise<JsonObject>;
  remember(scope: string, statement: string): Promise<JsonObject>;
  ingestText(scope: string, text: string, source?: string): Promise<JsonObject>;
  ingestJson(scope: string, document: string, source?: string): Promise<JsonObject>;
  ingestCsv(scope: string, document: string, source?: string): Promise<JsonObject>;
  ingestionJob(jobId: number): Promise<JsonObject>;
  validate(): Promise<JsonObject>;
  stats(): Promise<JsonObject>;
}
