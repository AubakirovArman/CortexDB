var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);
var cortexdb_client_exports = {};
__export(cortexdb_client_exports, {
  CortexDBClient: () => CortexDBClient,
  CortexDBError: () => CortexDBError
});
module.exports = __toCommonJS(cortexdb_client_exports);
class CortexDBError extends Error {
  constructor(message, code = null, status = null, body = null) {
    super(message);
    this.code = code;
    this.status = status;
    this.body = body;
    this.name = "CortexDBError";
  }
  code;
  status;
  body;
  static async fromResponse(response) {
    const body = await response.text();
    try {
      const data = JSON.parse(body);
      return new CortexDBError(
        String(data.message ?? body),
        data.code ? String(data.code) : null,
        response.status,
        body
      );
    } catch {
      return new CortexDBError(body, null, response.status, body);
    }
  }
}
class CortexDBClient {
  constructor(baseUrl = "http://127.0.0.1:8181", token, tenant, maxRetries = 0, retryDelayMs = 500) {
    this.baseUrl = baseUrl;
    this.token = token;
    this.tenant = tenant;
    this.maxRetries = maxRetries;
    this.retryDelayMs = retryDelayMs;
  }
  baseUrl;
  token;
  tenant;
  maxRetries;
  retryDelayMs;
  withTenant(tenant) {
    return new CortexDBClient(this.baseUrl, this.token, tenant, this.maxRetries, this.retryDelayMs);
  }
  withRetries(maxRetries, retryDelayMs = 500) {
    return new CortexDBClient(this.baseUrl, this.token, this.tenant, maxRetries, retryDelayMs);
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
      limit
    }));
  }
  searchVector(scope, vector, limit = 20, algorithm = "ann") {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "vector",
      algorithm,
      vector: vector.join(","),
      limit
    }));
  }
  evaluateAnn(scope, vector, limit = 20) {
    return this.request("POST", this.path("/v1/search/ann-evaluate", {
      scope,
      vector: vector.join(","),
      limit
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
    return this.request("POST", this.path("/v1/ingest/text", {
      scope,
      source
    }), text);
  }
  ingestJson(scope, document, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/json", {
      scope,
      source
    }), document);
  }
  ingestCsv(scope, document, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/csv", {
      scope,
      source
    }), document);
  }
  ingestionJob(jobId) {
    return this.request("GET", `/v1/ingest/jobs/${jobId}`);
  }
  ingestionJobResponse(jobId) {
    return this.request("GET", `/v1/ingest/jobs/${jobId}`);
  }
  deleteIngestionJob(jobId) {
    return this.request("DELETE", `/v1/ingest/jobs/${jobId}`);
  }
  retryIngestionJob(jobId) {
    return this.request("POST", `/v1/ingest/jobs/${jobId}/retry`);
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
    if (body !== void 0) {
      init.body = typeof body === "string" ? body : JSON.stringify(body);
      headers["content-type"] = "application/json";
    }
    const url = `${this.baseUrl}${this.scoped(path)}`;
    let attempt = 0;
    while (true) {
      try {
        const response = await fetch(url, init);
        if (!response.ok) {
          if (this.isRetryable(response.status) && attempt < this.maxRetries) {
            attempt += 1;
            await this.sleep(this.retryDelayMs * attempt);
            continue;
          }
          throw await CortexDBError.fromResponse(response);
        }
        return response.json();
      } catch (error) {
        if (error instanceof CortexDBError) throw error;
        if (attempt < this.maxRetries) {
          attempt += 1;
          await this.sleep(this.retryDelayMs * attempt);
          continue;
        }
        throw error;
      }
    }
  }
  isRetryable(status) {
    return [500, 502, 503, 504].includes(status);
  }
  sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
  path(path, query) {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      params.set(key, String(value));
    }
    return `${path}?${params.toString()}`;
  }
  scoped(path) {
    if (!this.tenant || this.tenant === "default") return path;
    const params = new URLSearchParams({ tenant: this.tenant });
    return `${path}${path.includes("?") ? "&" : "?"}${params.toString()}`;
  }
}
