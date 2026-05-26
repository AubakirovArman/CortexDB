export type JsonObject = Record<string, unknown>;

export class CortexDBClient {
  constructor(
    private readonly baseUrl = "http://127.0.0.1:8181",
    private readonly token?: string,
  ) {}

  putCell(cellId: number, payload: string): Promise<JsonObject> {
    return this.request("POST", this.path("/v1/cell", { cell_id: cellId }), payload);
  }

  getCell(cellId: number): Promise<JsonObject> {
    return this.request("GET", this.path("/v1/cell", { cell_id: cellId }));
  }

  search(scope: string, query: string, limit = 20): Promise<JsonObject> {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "keyword",
      q: query,
      limit,
    }));
  }

  searchVector(
    scope: string,
    vector: number[],
    limit = 20,
    algorithm: "ann" | "exact" = "ann",
  ): Promise<JsonObject> {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "vector",
      algorithm,
      vector: vector.join(","),
      limit,
    }));
  }

  aql(scope: string, statement: string): Promise<JsonObject> {
    return this.request("POST", this.path("/v1/aql", { scope }), statement);
  }

  retrieveContext(scope: string, statement: string): Promise<JsonObject> {
    return this.request("POST", this.path("/v1/context", { scope }), statement);
  }

  verifyFact(scope: string, statement: string): Promise<JsonObject> {
    return this.request("POST", this.path("/v1/verify", { scope }), statement);
  }

  remember(scope: string, statement: string): Promise<JsonObject> {
    return this.request("POST", this.path("/v1/remember", { scope }), statement);
  }

  validate(): Promise<JsonObject> {
    return this.request("GET", "/v1/validate");
  }

  stats(): Promise<JsonObject> {
    return this.request("GET", "/v1/stats");
  }

  private async request(method: string, path: string, body?: unknown): Promise<JsonObject> {
    const headers: Record<string, string> = {};
    if (this.token) headers.authorization = `Bearer ${this.token}`;
    const init: RequestInit = { method, headers };
    if (body !== undefined) {
      init.body = typeof body === "string" ? body : JSON.stringify(body);
      headers["content-type"] = "application/json";
    }
    const response = await fetch(`${this.baseUrl}${path}`, init);
    if (!response.ok) throw new Error(await response.text());
    return response.json() as Promise<JsonObject>;
  }

  private path(path: string, query: Record<string, string | number>): string {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      params.set(key, String(value));
    }
    return `${path}?${params.toString()}`;
  }
}
