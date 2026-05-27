# Tenant Naming Rules

CortexDB supports multi-tenant isolation through **realm paths**:

- `default` tenant maps to the database root directory.
- All other tenants map to `root/realms/<tenant>/`.

## Allowed Characters

A valid tenant ID must match:

```regex
^[a-zA-Z0-9_:-]{1,64}$
```

## Forbidden Patterns

The following are explicitly rejected:

| Pattern | Reason |
|---------|--------|
| Empty string | No directory name |
| `..` or `.` | Path traversal risk |
| `/` or `\` | Directory separators |
| `%` | URL-encoded separator risk |
| Length > 64 | Path length safety |
| `..%2f..` | URL-encoded traversal |

## Validation

Server validates tenant IDs before any filesystem access:

```rust
fn validate_tenant_id(tenant: &str) -> bool {
    !tenant.is_empty()
        && tenant.len() <= 64
        && tenant.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':'
        })
}
```

Invalid tenants receive HTTP `400 Bad Request` with code `invalid_tenant`.

## Examples

| Tenant ID | Valid | Directory |
|-----------|-------|-----------|
| `default` | ✅ | `./` |
| `project_1` | ✅ | `./realms/project_1/` |
| `tenant-alpha` | ✅ | `./realms/tenant-alpha/` |
| `org:team:service` | ✅ | `./realms/org:team:service/` |
| `../../etc` | ❌ | — |
| `a/b/c` | ❌ | — |
| `..%2f..` | ❌ | — |
