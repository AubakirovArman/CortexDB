# Lexical Index

Status: C01 compact persisted lexical index.

CortexDB writes lexical index files as `ACI4`. The runtime `LexicalIndex`
contract remains the same for query/search callers, but the persisted `.aci`
layout no longer repeats term strings in every posting and frequency section.

## ACI4 Contract

- A file starts with one sorted term dictionary.
- Posting lists reference dictionary entries by `term_id`.
- Candidate postings and frequency maps are sorted by candidate id.
- Candidate ids are stored as delta-varint streams.
- Term frequencies and field term frequencies store `term_id` plus compact
  candidate/frequency streams.
- `ACI0`, `ACI1`, `ACI2`, and `ACI3` remain read-only compatible.

This keeps search semantics stable while reducing persisted lexical-index
footprint for repeated long terms. `LexicalIndex::read_terms_only` still loads
only the term postings and doc lengths for large persisted index rebuilds.

## Migration

Offline migration and compaction rewrite persisted indexes through the current
writer, so old readable `ACI0..ACI3` files become `ACI4` after rewrite. No
in-place mutation of legacy index files is required.

## Gates

- `cargo test -p cortex-storage --test lexical_index_tests`
- `python3 scripts/lexical_index_contract_check.py`
- `make storage-format-freeze-check`
- `make migration-compatibility-check`
- `make storage-compat-check`
