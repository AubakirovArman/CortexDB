#!/usr/bin/env node
/**
 * TypeScript SDK smoke test against a live cortex-server.
 *
 * Validates typed response interfaces by issuing real requests.
 */
import { spawn } from "child_process";
import { existsSync, mkdtempSync, rmSync } from "fs";
import { tmpdir } from "os";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, "..");

// Dynamic import so we can point at the local SDK source
const { CortexDBClient } = await import(
  join(repoRoot, "sdk/typescript/cortexdb-client.ts")
    .replace(/\.ts$/, ".js")
);

const PORT = 18184;
const dbDir = mkdtempSync(join(tmpdir(), "cortex_ts_smoke_"));
const AUTH_TOKEN = "sdk-smoke-secret";

function findBinary() {
  if (process.env.CORTEXDB_SERVER_BIN) {
    return process.env.CORTEXDB_SERVER_BIN;
  }
  const release = join(repoRoot, "target/release/cortex-server");
  const debug = join(repoRoot, "target/debug/cortex-server");
  if (existsSync(release)) {
    return release;
  }
  return debug;
}

function waitForServer(port, timeoutMs = 10000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve) => {
    const tryConnect = () => {
      fetch(`http://127.0.0.1:${port}/v1/health`, { signal: AbortSignal.timeout(200) })
        .then(() => resolve(true))
        .catch(() => {
          if (Date.now() >= deadline) resolve(false);
          else setTimeout(tryConnect, 100);
        });
    };
    tryConnect();
  });
}

async function main() {
  const binary = findBinary();
  const server = spawn(binary, [dbDir, `127.0.0.1:${PORT}`], {
    stdio: "ignore",
    env: { ...process.env, RUST_LOG: "error", CORTEXDB_AUTH_TOKEN: AUTH_TOKEN },
  });

  const ready = await waitForServer(PORT);
  if (!ready) {
    console.error("Server did not start");
    server.kill();
    process.exit(1);
  }

  const baseUrl = `http://127.0.0.1:${PORT}`;
  const client = new CortexDBClient(baseUrl, AUTH_TOKEN);
  let failed = false;

  function assertEqual(actual, expected, label) {
    if (actual !== expected) {
      console.error(`FAIL ${label}: expected ${expected}, got ${actual}`);
      failed = true;
    }
  }

  async function expectError(label, action, status, code) {
    try {
      await action();
      console.error(`FAIL ${label}: expected CortexDBError`);
      failed = true;
    } catch (err) {
      assertEqual(err.status, status, `${label}.status`);
      assertEqual(err.code, code, `${label}.code`);
      if (!err.body) {
        console.error(`FAIL ${label}.body missing`);
        failed = true;
      }
      console.log(`OK: ${label}`);
    }
  }

  try {
    await expectError(
      "missing_auth_error_contract",
      () => new CortexDBClient(baseUrl).health(),
      401,
      "unauthorized",
    );

    // Health
    const health = await client.health();
    assertEqual(health.status, "ok", "health.status");
    assertEqual(health.version, "v1", "health.version");
    if (!health.server_version) {
      console.error("FAIL health.server_version missing");
      failed = true;
    }
    console.log("OK: health");

    // Put cell
    const put = await client.putCell(1, "scope=default\nstatus=ready\ntype=fact\nsource=smoke\n\nhello world");
    assertEqual(put.seq, 1, "put.seq");
    assertEqual(put.cell_id, 1, "put.cell_id");
    console.log("OK: putCell");

    // Get cell
    const lookup = await client.getCell(1);
    if (!lookup.cell || lookup.cell.cell_id !== 1) {
      console.error("FAIL getCell");
      failed = true;
    }
    console.log("OK: getCell");

    // Search
    const search = await client.search("default", "hello", 10);
    assertEqual(search.search_mode, "keyword", "search.search_mode");
    console.log("OK: search");

    // Stats
    const stats = await client.stats();
    if (stats.current_seq < 1) {
      console.error("FAIL stats.current_seq");
      failed = true;
    }
    console.log("OK: stats");

    // Validate
    const validation = await client.validate();
    assertEqual(validation.ok, true, "validate.ok");
    console.log("OK: validate");

    // AQL
    const aql = await client.aql("default", 'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;');
    if (!Array.isArray(aql.cells)) {
      console.error("FAIL aql.cells");
      failed = true;
    }
    console.log("OK: aql");

    // Context
    const context = await client.retrieveContext("default", 'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;');
    if (typeof context.token_budget_tokens !== "number") {
      console.error("FAIL context.token_budget_tokens");
      failed = true;
    }
    console.log("OK: retrieveContext");

    // Verify
    const verify = await client.verifyFact("default", 'VERIFY FACT "hello world" IN BRAIN default;');
    assertEqual(verify.fact, "hello world", "verify.fact");
    console.log("OK: verifyFact");

    // Remember
    const remember = await client.remember("default", 'REMEMBER "test memory" IN SCOPE default AS TYPE decision TTL 3600 SECONDS;');
    if (remember.seq <= 0) {
      console.error("FAIL remember.seq");
      failed = true;
    }
    console.log("OK: remember");

    // Ingest
    const ingest = await client.ingestText("default", "hello world ingestion");
    if (ingest.chunks_ingested < 1) {
      console.error("FAIL ingest.chunks_ingested");
      failed = true;
    }
    console.log("OK: ingestText");

    // Error contract
    await expectError(
      "invalid_aql_error_contract",
      () => client.aql(
        "default",
        'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default USING MODE turbo;',
      ),
      400,
      "invalid_aql",
    );
    await expectError(
      "not_found_error_contract",
      () => client.ingestionJobResponse(999999),
      404,
      "not_found",
    );
    await expectError(
      "invalid_tenant_error_contract",
      () => client.withTenant("../bad").stats(),
      400,
      "invalid_tenant",
    );
  } catch (err) {
    console.error(`FAIL: ${err.message || err}`);
    failed = true;
  } finally {
    server.kill();
    rmSync(dbDir, { recursive: true, force: true });
  }

  if (failed) {
    process.exit(1);
  }
  console.log("\nAll TypeScript SDK smoke tests passed.");
}

main();
