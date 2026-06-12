# Managed Cloud Feasibility Track

Status: feasibility evidence track, not a hosted managed service.

CortexDB currently remains a local single-node database plus local server
surface. The managed-cloud track records prerequisite evidence for a possible
future hosted product.

## Aggregate Gate

Run:

```bash
make managed-cloud-feasibility-check
```

The aggregate gate depends on:

```text
make cloud-tenant-lifecycle-check
make cloud-backup-restore-check
make cloud-upgrade-check
```

Generated reports:

```text
target/managed-cloud/tenant-lifecycle.json
target/managed-cloud/backup-restore.json
target/managed-cloud/upgrade.json
target/managed-cloud/feasibility-summary.json
```

Every managed-cloud report must keep:

```text
managed_cloud_ready=false
```

## Covered Prerequisites

- Tenant routing and invalid-tenant fail-closed behavior.
- Tenant backup/restore isolation.
- HTTP contract and security checks.
- Observability report shape.
- Local backup drill and offsite staging.
- Deployment upgrade and migration compatibility gates.

## Not Covered Yet

- Hosted control plane.
- Tenant provisioning and deletion automation.
- Cloud object-store backup and restore.
- Billing and quota accounting.
- Support break-glass workflows.
- Multi-tenant production SLOs.
- Public managed service operations.

## Promotion Requirement

Before CortexDB can claim managed-cloud readiness, the feasibility track must be
replaced by a real staging environment with:

1. tenant create/suspend/resume/delete workflows;
2. cloud backup storage and restore drill;
3. quota and billing event reports;
4. operator support workflow with audited access;
5. upgrade and rollback against hosted tenant instances.
