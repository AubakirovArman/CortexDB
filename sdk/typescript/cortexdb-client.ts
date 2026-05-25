export class CortexDBClient {
  constructor(
    private readonly baseUrl = "http://127.0.0.1:8181",
    private readonly token?: string,
  ) {}

  putCell(cellId: number, payload: string): Promise<unknown> {
    return this.request("POST", `/v1/cell?cell_id=${cellId}`, payload);
  }

  getCell(cellId: number): Promise<unknown> {
    return this.request("GET", `/v1/cell?cell_id=${cellId}`);
  }

  search(scope: string, query: string, limit = 20): Promise<unknown> {
    return this.request("POST", "/v1/search", { scope, query, limit });
  }

  retrieveContext(scope: string, aql: string): Promise<unknown> {
    return this.request("POST", `/v1/context?scope=${scope}`, aql);
  }

  verifyFact(scope: string, aql: string): Promise<unknown> {
    return this.request("POST", `/v1/verify?scope=${scope}`, aql);
  }

  remember(scope: string, aql: string): Promise<unknown> {
    return this.request("POST", `/v1/remember?scope=${scope}`, aql);
  }

  validate(): Promise<unknown> {
    return this.request("GET", "/v1/validate");
  }

  stats(): Promise<unknown> {
    return this.request("GET", "/v1/stats");
  }

  private async request(method: string, path: string, body?: unknown): Promise<unknown> {
    const headers: Record<string, string> = {};
    if (this.token) headers.authorization = `Bearer ${this.token}`;
    const init: RequestInit = { method, headers };
    if (body !== undefined) {
      init.body = typeof body === "string" ? body : JSON.stringify(body);
      headers["content-type"] = "application/json";
    }
    const response = await fetch(`${this.baseUrl}${path}`, init);
    if (!response.ok) throw new Error(await response.text());
    return response.json();
  }
}
