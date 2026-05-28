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
  CortexDBClient: () => CortexDBClient
});
module.exports = __toCommonJS(cortexdb_client_exports);
class CortexDBClient {
  constructor(baseUrl = "http://127.0.0.1:8181", token, tenant) {
    this.baseUrl = baseUrl;
    this.token = token;
    this.tenant = tenant;
  }
  baseUrl;
  token;
  tenant;
  withTenant(tenant) {
    return new CortexDBClient(this.baseUrl, this.token, tenant);
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
    const response = await fetch(`${this.baseUrl}${this.scoped(path)}`, init);
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
  scoped(path) {
    if (!this.tenant || this.tenant === "default") return path;
    const params = new URLSearchParams({ tenant: this.tenant });
    return `${path}${path.includes("?") ? "&" : "?"}${params.toString()}`;
  }
}
