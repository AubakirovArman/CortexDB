# CortexDB Incident Playbooks

Status: local single-node incident playbooks for the current production epic
plan. These playbooks are operator runbooks, not managed-cloud paging,
multi-node failover, or enterprise compliance procedures.

Use [`OPERATIONS_RUNBOOK_V1.md`](OPERATIONS_RUNBOOK_V1.md) for the normal
startup, shutdown, backup, restore, repair, and upgrade sequence. Use this file
when an alert or user-visible problem is already happening.

## Shared Rules

1. Save evidence before changing state.
2. Prefer read-only commands first.
3. Stop release promotion on validation, corruption, backup, auth, or tenant
   incidents until the playbook exit criteria pass.
4. Restore from the latest validated backup when repair cannot produce a safe
   plan.
5. Do not delete WAL, segment, index, manifest, audit, or backup files outside
   documented CLI repair or restore commands.

Shared evidence commands:

```bash
cortexdb validate ./data
cortexdb stats ./data
cortexdb doctor ./data
cortexdb wal-validate ./data
curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/validate
curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/metrics
```

## Playbook 1. Corrupted Storage

Trigger examples:

- `cortexdb validate ./data` reports segment, bitmap, lexical, manifest, WAL, or
  checksum failure;
- `/v1/validate` returns a corruption or validation error;
- alert `CortexDbValidationFailures` fires;
- `cortexdb doctor ./data` recommends repair or restore.

Triage:

```bash
cortexdb validate ./data > incident-validate.txt
cortexdb stats ./data > incident-stats.txt
cortexdb wal-validate ./data > incident-wal.txt
cortexdb manifest-validate ./data > incident-manifest.txt
cortexdb repair ./data --dry-run > incident-repair-plan.txt
```

Containment:

1. Stop writers and the HTTP server.
2. Keep the database directory intact for evidence.
3. Do not run `wal-truncate` until dry-run repair or recovery evidence says the
   safe offset is valid.

Recovery:

```bash
cortexdb repair ./data --best-effort
cortexdb validate ./data
```

If validation still fails:

```bash
cortexdb restore ./backups/latest-validated ./data-restored --dry-run
cortexdb restore ./backups/latest-validated ./data-restored
cortexdb validate ./data-restored
```

Exit criteria:

- `cortexdb validate` passes;
- restored or repaired data path is selected deliberately;
- release promotion evidence is rerun before publishing.

## Playbook 2. Actor Busy

Trigger examples:

- `/v1/*` returns `database_busy`;
- `cortexdb_request_rejected` increases;
- alert `CortexDbActorQueuePressure` or `CortexDbDatabaseBusy` fires;
- `actor_queue_depth` approaches `actor_queue_capacity`.

Triage:

```bash
curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/metrics > incident-metrics.txt
cortexdb stats ./data > incident-stats.txt
cortexdb validate ./data > incident-validate.txt
```

Containment:

1. Reduce caller concurrency.
2. Add retry backoff with jitter.
3. Avoid tight retry loops after `database_busy`.
4. Do not raise `--actor-queue-capacity` until WAL and validation are healthy.

Recovery:

```bash
make load-smoke-check
cortexdb validate ./data
```

Exit criteria:

- queue depth returns below the configured alert threshold;
- request rejection rate returns to expected baseline;
- validation still passes after the pressure event.

## Playbook 3. Backup Failed

Trigger examples:

- `cortexdb backup`, `backup-drill`, `backup-encrypted`, or
  `backup-offsite-stage` fails;
- alert `CortexDbBackupStale` or `CortexDbBackupEvidenceMissing` fires;
- backup destination is unavailable or restore drill fails.

Triage:

```bash
cortexdb validate ./data > incident-validate.txt
cortexdb stats ./data > incident-stats.txt
cortexdb backup-drill ./data ./backups/drill ./drills/drill-restored > incident-backup-drill.txt
```

Containment:

1. Stop release promotion.
2. Keep the last validated backup until a new restore drill passes.
3. Do not prune backup generations during the incident.

Recovery:

```bash
cortexdb backup ./data ./backups/cortexdb-$(date -u +%Y%m%dT%H%M%SZ)
cortexdb backup-drill ./data ./backups/drill ./drills/drill-restored
cortexdb backup-offsite-stage ./backups/cortexdb-20260602 ./offsite cortexdb-20260602
make backup-restore-production-pack-check
```

Exit criteria:

- local backup succeeds;
- restore drill succeeds;
- offsite staging succeeds when offsite evidence is required;
- backup age and evidence metrics are no longer stale or missing.

## Playbook 4. Auth Failure Spike

Trigger examples:

- repeated `401`, `403`, or auth-denied error responses;
- `cortexdb_request_count` rises while protected route success drops;
- audit log shows many denied principals, invalid bearer tokens, or missing
  capabilities.

Triage:

```bash
curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/metrics > incident-metrics.txt
cortexdb audit ./audit/http.jsonl --summary --redaction-check > incident-audit-summary.txt
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain > incident-audit-chain.txt
```

Containment:

1. Rotate affected tokens if compromise is suspected.
2. Disable or remove the affected principal in the policy store when dynamic
   policy store is enabled.
3. Keep audit exports redacted; do not paste raw tokens into tickets.

Recovery:

```bash
cortexdb auth-review ./auth/policy.json --summary
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain
make security-gate-v2-check
```

Exit criteria:

- expected principals can authenticate;
- denied spike stops;
- audit chain verifies;
- redaction checks pass.

## Playbook 5. Tenant Issue

Trigger examples:

- route returns `invalid_tenant`;
- tenant path traversal attempt is detected;
- principal is mapped to an unexpected tenant or scope;
- tenant-specific validation or backup evidence is missing.

Triage:

```bash
cortexdb validate ./data --tenant tenant-alpha > incident-tenant-validate.txt
curl -H "Authorization: Bearer dev-token" "http://127.0.0.1:8181/v1/validate?tenant=tenant-alpha"
cortexdb audit ./audit/http.jsonl --summary --redaction-check > incident-tenant-audit.txt
```

Containment:

1. Stop writes from the affected principal.
2. Verify tenant names against [`TENANT_NAMING_RULES.md`](TENANT_NAMING_RULES.md).
3. Do not map provider group names directly to tenant scope without policy
   validation.

Recovery:

```bash
make tenant-recovery-check
make quota-policy-check
make security-check
```

Exit criteria:

- tenant validation passes;
- affected principal maps only to intended tenant and scopes;
- tenant recovery and quota policy gates pass.

## Evidence Gate

Run the playbook gate after editing this document:

```bash
make incident-playbooks-check
```

The gate verifies that each playbook has triggers, triage, containment,
recovery, and exit criteria, and that command markers for validation, repair,
backup, metrics, audit, and tenant recovery are present.
