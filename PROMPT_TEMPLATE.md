# Request template for CortexDB work

Use this block for every non-trivial request:

**Role:** `Fix/implement/verify ...`
**Scope:** `#file path/to/file.rs`, `#file path/to/other.rs`
**Goal:** one sentence of expected behavior
**Constraints:**
- do:
  - keep backward compatibility for ...
  - add/adjust tests in ...
  - keep output format stable
- do-not:
  - change public contract ...
  - add new dependencies
  - introduce floating-point scoring
**Output format:**
1. What changed
2. Why
3. Validation commands + results
4. Next step
**Success:** exact command list
- `cargo fmt --check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`
