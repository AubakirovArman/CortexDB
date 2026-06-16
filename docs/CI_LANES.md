# CI Lanes

CortexDB CI is split by cost and release risk.

| Lane | Trigger | Checks |
| --- | --- | --- |
| PR | `pull_request`, `push` to `main` | file-size, docs-link, stable `cargo check`, full workspace tests, fmt, clippy, live examples, migration policy, storage-format change note, EnterpriseRAG fixture quality/parity/query-understanding gates |
| Nightly | daily schedule, manual dispatch | beta toolchain check/test/fmt/clippy, load smoke, crash/fault, backup offsite, chaos restart, replication partition/lifecycle, dashboard package/smoke/screenshots, ANN regression/release evidence, continuous scale benchmark gate |
| Release | `v*` tag, manual dispatch on tag | full `make release-check`, release evidence bundle, ANN baseline package, container image, binary release artifacts |

Specialized ANN and continuous benchmark workflows remain manual entry points,
but their scheduled coverage is owned by `nightly.yml`.
