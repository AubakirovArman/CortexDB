# Benchmarks

Core Alpha includes a dependency-free benchmark baseline for the single-node
engine loop.

Run it with:

```bash
cargo bench -p cortex-engine --bench core_baseline
```

The benchmark is intentionally small and deterministic. It runs in a temporary
database directory and measures:

- `put` for 256 cells through WAL and MemTable
- `get` for 256 latest-cell reads
- incremental checkpoint for those cells
- reopen from checkpoint
- AQL-backed ContextPack creation

Example output from the current workstation:

```text
cortexdb core baseline
put 256 cells: 127.136098ms
get 256 cells: 56.482us
checkpoint 256 cells: 7.335105ms
reopen from checkpoint: 1.066443ms
context pack: 926.426us
```

These numbers are a baseline smoke signal, not a performance guarantee. The
release gate is that the benchmark compiles and runs without external services.
