#!/usr/bin/env node
/**
 * TypeScript SDK smoke test against a live cortex-server.
 *
 * Validates typed response interfaces by issuing real requests.
 */
import { spawn } from "child_process";
import { mkdtempSync, rmSync } from "fs";
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

function findBinary() {
  const release = join(repoRoot, "target/release/cortex-server");
  const debug = join(repoRoot, "target/debug/cortex-server");
  try {
    // eslint-disable-next-line no-sync
    import("fs").then(({ statSync }) => statSync(release));
    return release;
  } catch {
    return debug;
  }
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
    env: { ...process.env, RUST_LOG: "error" },
  });

  const ready = await waitForServer(PORT);
  if (!ready) {
    console.error("Server did not start");
    server.kill();
    process.exit(1);
  }

  const client = new CortexDBClient(`http://127.0.0.1:${PORT}`);
  let failed = false;

  function assertEqual(actual, expected, label) {
    if (actual !== expected) {
      console.error(`FAIL ${label}: expected ${expected}, got ${actual}`);
      failed = true;
    }
  }

  try {
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
