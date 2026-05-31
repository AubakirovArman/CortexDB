# Tenant Naming Rules

CortexDB supports multi-tenant isolation through **realm paths**:

- `default` tenant maps to the database root directory.
- All other tenants map to `root/realms/<tenant>/`.

## Allowed Characters

A valid tenant ID must match:

```regex
^[a-zA-Z0-9_-]{1,64}$
```

**Note:** `:` is intentionally disallowed in tenant IDs to ensure cross-platform
safety (Windows reserves `:` in file paths), even though Linux permits it.
Scope names may still contain `:` (e.g. `project:investments`), but tenant IDs may not.

## Forbidden Patterns

The following are explicitly rejected:

| Pattern | Reason |
|---------|--------|
| Empty string | No directory name |
| `..` or `.` | Path traversal risk |
| `/` or `\` | Directory separators |
| `:` | Windows file path reserved character |
| `%` | URL-encoded separator risk |
| Length > 64 | Path length safety |
| `..%2f..` | URL-encoded traversal |

## Validation

Server validates tenant IDs **after percent-decoding** the query parameter,
before any filesystem access:

```rust
fn validate_tenant_id(tenant: &str) -> bool {
    !tenant.is_empty()
        && tenant.len() <= 64
        && tenant.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '_' || c == '-'
        })
}
```

Invalid tenants receive HTTP `400 Bad Request` with code `invalid_tenant`.

## Recovery Gate

Run:

```bash
make tenant-recovery-check
```

The gate writes `target/tenant-recovery/report.json` and verifies that
`default`, `tenant-alpha`, and `tenant-beta` remain isolated across
flush/validate, backup, restore, and server restart.

## Examples

| Tenant ID | Valid | Directory |
|-----------|-------|-----------|
| `default` | ✅ | `./` |
| `project_1` | ✅ | `./realms/project_1/` |
| `tenant-alpha` | ✅ | `./realms/tenant-alpha/` |
| `org_team_service` | ✅ | `./realms/org_team_service/` |
| `../../etc` | ❌ | — |
| `a/b/c` | ❌ | — |
| `org:team:service` | ❌ | — |
| `..%2f..` | ❌ | — |
