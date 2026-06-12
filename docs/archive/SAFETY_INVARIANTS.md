# Safety Invariants

- AQL filters may only narrow an `AgentView`; they must never grant access.
- `NOT` over bitmaps is evaluated inside the segment-local universe.
- Policy deny errors are fail-closed.
- Operational limits may be clamped only when policy marks them as clampable.
- Semantic constraints such as invalid confidence thresholds are not silently clamped.
- Core scoring and persistent formats avoid floating-point state.
- WAL recovery stops at corrupt or partial tails and reports a safe truncate offset.
- Unknown WAL section tags must be skipped or preserved safely.
