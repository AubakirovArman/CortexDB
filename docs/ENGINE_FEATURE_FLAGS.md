# Engine Feature Boundaries

CortexDB separates compile-time experimental code from runtime activation.
Stable single-node engine consumers do not compile the replication modules by
default.

## Replication

Replication requires both boundaries:

1. Cargo feature `experimental-replication` exposes the `distributed` and
   `replication` modules.
2. Runtime flag `experimental_replication` (or
   `CORTEXDB_EXPERIMENTAL_REPLICATION=true`) permits mutation of the guarded
   replication surface.

`cortex-server` compiles the feature because it exposes research cluster-status
and ingress surfaces, but runtime activation remains off by default. This is an
experimental compile boundary, not a production HA claim.

## HNSW And Dashboard

`experimental_hnsw` and `CORTEXDB_DASHBOARD=true` remain runtime opt-ins.
Default `DatabaseOptions` must not build HNSW graphs, activate replication, or
serve dashboard assets.
