export class CortexDBClient {
  constructor(baseUrl = "http://127.0.0.1:8181", token) {
    this.baseUrl = baseUrl;
    this.token = token;
  }

  health() {
    return this.request("GET", "/v1/health");
  }

  putCell(cellId, payload) {
    return this.request("POST", this.path("/v1/cell", { cell_id: cellId }), payload);
  }

  getCell(cellId) {
    return this.request("GET", this.path("/v1/cell", { cell_id: cellId }));
  }

  tombstoneCell(cellId) {
    return this.request("DELETE", this.path("/v1/cell", { cell_id: cellId }));
  }

  flush() {
    return this.request("POST", "/v1/flush");
  }

  compact() {
    return this.request("POST", "/v1/compact");
  }

  search(scope, query, limit = 20) {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "keyword",
      q: query,
      limit,
    }));
  }

  searchVector(scope, vector, limit = 20, algorithm = "ann") {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "vector",
      algorithm,
      vector: vector.join(","),
      limit,
    }));
  }

  evaluateAnn(scope, vector, limit = 20) {
    return this.request("POST", this.path("/v1/search/ann-evaluate", {
      scope,
      vector: vector.join(","),
      limit,
    }));
  }

  aql(scope, statement) {
    return this.request("POST", this.path("/v1/aql", { scope }), statement);
  }

  retrieveContext(scope, statement) {
    return this.request("POST", this.path("/v1/context", { scope }), statement);
  }

  verifyFact(scope, statement) {
    return this.request("POST", this.path("/v1/verify", { scope }), statement);
  }

  remember(scope, statement) {
    return this.request("POST", this.path("/v1/remember", { scope }), statement);
  }

  ingestText(scope, text, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/text", { scope, source }), text);
  }

  ingestJson(scope, document, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/json", { scope, source }), document);
  }

  ingestCsv(scope, document, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/csv", { scope, source }), document);
  }

  ingestionJob(jobId) {
    return this.request("GET", `/v1/ingest/jobs/${jobId}`);
  }

  validate() {
    return this.request("GET", "/v1/validate");
  }

  stats() {
    return this.request("GET", "/v1/stats");
  }

  async request(method, path, body) {
    const headers = {};
    if (this.token) headers.authorization = `Bearer ${this.token}`;
    const init = { method, headers };
    if (body !== undefined) {
      init.body = typeof body === "string" ? body : JSON.stringify(body);
      headers["content-type"] = "application/json";
    }
    const response = await fetch(`${this.baseUrl}${path}`, init);
    if (!response.ok) throw new Error(await response.text());
    return response.json();
  }

  path(path, query) {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      params.set(key, String(value));
    }
    return `${path}?${params.toString()}`;
  }
}
