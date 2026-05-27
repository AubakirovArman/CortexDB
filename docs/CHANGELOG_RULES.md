# Changelog Rules

## Format

Each release section follows this structure:

```markdown
## vX.Y.Z — Title

### Added
- New features.

### Changed
- Changes to existing behavior.

### Deprecated
- Features marked for removal.

### Removed
- Deleted features.

### Fixed
- Bug fixes.

### Security
- Security fixes.
```

## Categories

| Category | When to use |
|----------|-------------|
| `Added` | New endpoints, fields, commands, or SDK methods. |
| `Changed` | Behavioral changes that are NOT breaking for clients. |
| `Deprecated` | Features that will be removed in a future version. |
| `Removed` | Deleted endpoints, fields, or commands. |
| `Fixed` | Bug fixes. |
| `Security` | CVEs, auth fixes, permission fixes. |

## Version Bumping

| Change type | Version bump |
|-------------|--------------|
| Breaking API change | **Major** (`0.2.0` → `0.3.0`) |
| New feature, non-breaking | **Minor** (`0.2.0` → `0.2.1`) |
| Bug fix only | **Patch** (`0.2.0` → `0.2.0-post1`) |

> Pre-1.0: minor bumps may contain breaking changes. After 1.0, SemVer is strict.

## PR Requirements

Every PR that changes user-facing behavior MUST update `CHANGELOG.md`.

Internal-only changes (refactors, test-only changes, docs fixes without behavior change) may skip the changelog with `[skip changelog]` in the PR description.

## Unreleased Section

The top of `CHANGELOG.md` always contains an `## Unreleased` section. When a release is cut, this section is renamed to the release version and date.
