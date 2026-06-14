# Engine Config

`EngineConfig` is the env-driven loader for embedded engine startup settings.
It produces `DatabaseOptions` for callers that want the same configuration
surface as the CLI and server.

Stable options include durability recovery mode, stale lock policy, compaction
policy, payload residency, feature flags, and the text analyzer profile.

`DatabaseOptions::text_analyzer` defaults to a neutral analyzer with stemming
disabled. Non-default analyzer profiles are persisted in the storage manifest
after checkpoint/compact and must match on reopen while persisted segments
exist.
