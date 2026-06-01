# Managed Cloud Design

Status: future design gate, not implemented.

## Goal

Define what must exist before CortexDB can be operated as a managed hosted
service.

## Control Plane

The control plane owns tenant provisioning, deletion, suspension, backup
policies, upgrades, quota policy, and support access. It must never directly
mutate tenant data files without a documented data-plane operation.

## Data Plane

The data plane runs isolated CortexDB instances or shards. It must expose
health, metrics, backup, restore, upgrade, and tenant-lifecycle hooks to the
control plane.

## Tenant Lifecycle

The lifecycle must cover:

1. create tenant;
2. bind auth and AgentView policy;
3. ingest and retrieve;
4. backup and restore;
5. suspend and resume;
6. delete with retention policy;
7. prove deletion or retention state.

## Billing And Quotas

The cloud service needs usage counters for storage bytes, WAL bytes, requests,
ContextPack calls, Verify calls, ANN evaluations, backup storage, and egress.

Quota exhaustion must return typed errors and must not corrupt local database
state.

## Support And Break-glass

Support workflows need audited access, approval policy, limited duration, and
redacted logs. Break-glass access must be opt-in and must create audit events.

## Required Gates

1. `make managed-cloud-design-check`
2. `make cloud-tenant-lifecycle-check`
3. `make cloud-backup-restore-check`
4. `make cloud-upgrade-check`
5. `make public-claims-check`

## Acceptance

1. A staging cloud can provision, use, back up, restore, and delete a tenant.
2. Tenant data does not leak across boundaries.
3. Billing and quota events are observable.
4. Upgrade and rollback are documented and tested.

## Non-goals

1. Claiming managed cloud readiness from local single-node evidence.
2. Implementing billing before tenant isolation is proven.
3. Support access without auditable approval.
