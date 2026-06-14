use serde::Serialize;

use cortex_storage::format::storage_format_specs;

const MATRIX_JSON: &str = include_str!("../../../fixtures/migration/compatibility_matrix_v1.json");

#[derive(Clone, Debug, Serialize)]
pub struct CompatibilitySummary {
    pub schema_version: &'static str,
    pub api: ApiCompatibility,
    pub sdk: SdkCompatibility,
    pub storage_formats: Vec<StorageFormatCompatibility>,
    pub migration: MigrationCompatibility,
    pub migration_registry: MigrationVersionRegistry,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiCompatibility {
    pub version: &'static str,
    pub contract: &'static str,
    pub gate: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SdkCompatibility {
    pub contract: &'static str,
    pub workspace_version: &'static str,
    pub gate: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct StorageFormatCompatibility {
    pub name: &'static str,
    pub extension: &'static str,
    pub current_magic: String,
    pub current_version: u16,
    pub legacy_magics: Vec<String>,
    pub compatibility_rule: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct MigrationCompatibility {
    pub matrix_schema_version: u64,
    pub release: String,
    pub current_release: String,
    pub gate: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct MigrationVersionRegistry {
    pub schema_version: &'static str,
    pub current_release: String,
    pub migration_gate: &'static str,
    pub downgrade_policy: String,
    pub formats: Vec<MigrationFormatRegistryEntry>,
    pub releases: Vec<MigrationReleaseRegistryEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MigrationFormatRegistryEntry {
    pub kind: &'static str,
    pub extension: &'static str,
    pub current_magic: String,
    pub current_version: u16,
    pub legacy_magics: Vec<String>,
    pub compatibility_rule: &'static str,
    pub required_gate: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct MigrationReleaseRegistryEntry {
    pub from: String,
    pub to: String,
    pub mode: String,
    pub downgrade: String,
    pub fixture: Option<String>,
    pub backup_path: Option<String>,
    pub database_path: Option<String>,
}

pub fn compatibility_summary() -> CompatibilitySummary {
    let matrix: serde_json::Value =
        serde_json::from_str(MATRIX_JSON).unwrap_or(serde_json::Value::Null);
    let storage_formats = storage_format_specs();
    let migration_registry = migration_version_registry(&matrix, &storage_formats);
    CompatibilitySummary {
        schema_version: "cortexdb.compatibility.v1",
        api: ApiCompatibility {
            version: "v1",
            contract: "openapi.v1",
            gate: "make openapi-contract-check",
        },
        sdk: SdkCompatibility {
            contract: "sdk-contract.v1",
            workspace_version: env!("CARGO_PKG_VERSION"),
            gate: "make sdk-contract-check",
        },
        storage_formats: storage_formats
            .iter()
            .map(|spec| StorageFormatCompatibility {
                name: spec.name,
                extension: spec.extension,
                current_magic: magic_to_string(spec.current_magic),
                current_version: spec.current_version,
                legacy_magics: spec
                    .legacy_magics
                    .iter()
                    .map(|magic| magic_to_string(magic))
                    .collect(),
                compatibility_rule: spec.compatibility_rule,
            })
            .collect(),
        migration: MigrationCompatibility {
            matrix_schema_version: matrix
                .get("schema_version")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            release: string_field(&matrix, "release"),
            current_release: string_field(&matrix, "current_release"),
            gate: "make migration-compatibility-check",
        },
        migration_registry,
    }
}

fn migration_version_registry(
    matrix: &serde_json::Value,
    storage_formats: &[cortex_storage::format::StorageFormatSpec],
) -> MigrationVersionRegistry {
    MigrationVersionRegistry {
        schema_version: "cortexdb.migration_registry.v1",
        current_release: string_field(matrix, "current_release"),
        migration_gate: "make migration-compatibility-check",
        downgrade_policy: downgrade_policy(matrix),
        formats: storage_formats
            .iter()
            .map(|spec| MigrationFormatRegistryEntry {
                kind: storage_format_kind_name(spec.kind),
                extension: spec.extension,
                current_magic: magic_to_string(spec.current_magic),
                current_version: spec.current_version,
                legacy_magics: spec
                    .legacy_magics
                    .iter()
                    .map(|magic| magic_to_string(magic))
                    .collect(),
                compatibility_rule: spec.compatibility_rule,
                required_gate: "make storage-format-freeze-check",
            })
            .collect(),
        releases: release_registry(matrix),
    }
}

fn storage_format_kind_name(kind: cortex_storage::format::StorageFormatKind) -> &'static str {
    match kind {
        cortex_storage::format::StorageFormatKind::AclogWal => "aclog_wal",
        cortex_storage::format::StorageFormatKind::Segment => "segment",
        cortex_storage::format::StorageFormatKind::BitmapIndex => "bitmap_index",
        cortex_storage::format::StorageFormatKind::LexicalIndex => "lexical_index",
        cortex_storage::format::StorageFormatKind::VectorIndex => "vector_index",
        cortex_storage::format::StorageFormatKind::HnswGraph => "hnsw_graph",
        cortex_storage::format::StorageFormatKind::Manifest => "manifest",
    }
}

fn downgrade_policy(matrix: &serde_json::Value) -> String {
    matrix
        .get("upgrade_matrix")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("downgrade"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("restore-only from immutable pre-upgrade backup; no in-place downgrade")
        .to_owned()
}

fn release_registry(matrix: &serde_json::Value) -> Vec<MigrationReleaseRegistryEntry> {
    matrix
        .get("release_compatibility_matrix")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .map(|item| MigrationReleaseRegistryEntry {
            from: string_field(item, "from"),
            to: string_field(item, "to"),
            mode: string_field(item, "mode"),
            downgrade: string_field(item, "downgrade"),
            fixture: optional_string_field(item, "fixture"),
            backup_path: optional_string_field(item, "backup_path"),
            database_path: optional_string_field(item, "database_path"),
        })
        .collect()
}

fn magic_to_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |index| index + 1);
    let trimmed = &bytes[..end];
    String::from_utf8_lossy(trimmed).into_owned()
}

fn string_field(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_owned()
}

fn optional_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_summary_exposes_public_contracts() {
        let summary = compatibility_summary();
        assert_eq!(summary.schema_version, "cortexdb.compatibility.v1");
        assert_eq!(summary.api.version, "v1");
        assert_eq!(summary.sdk.contract, "sdk-contract.v1");
        assert!(summary
            .storage_formats
            .iter()
            .any(|format| format.current_magic == "ACLOGv0"));
        assert_eq!(summary.migration.gate, "make migration-compatibility-check");
        assert_eq!(
            summary.migration_registry.schema_version,
            "cortexdb.migration_registry.v1"
        );
    }

    #[test]
    fn compatibility_summary_exposes_all_storage_format_markers() {
        let summary = compatibility_summary();
        let markers = summary
            .storage_formats
            .iter()
            .map(|format| format.current_magic.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            markers,
            vec!["ACLOGv0", "ACS3", "ACB1", "ACI3", "ACV0", "ACH0", "ACM0"]
        );

        let segment = summary
            .storage_formats
            .iter()
            .find(|format| format.extension == "acs")
            .expect("segment compatibility should be public");
        assert_eq!(segment.current_version, 3);
        assert_eq!(segment.legacy_magics, vec!["ACS1", "ACS2"]);
        assert_eq!(
            segment.compatibility_rule,
            "ACS1 and ACS2 remain read-only compatible; breaking changes require a new segment magic"
        );

        let bitmap = summary
            .storage_formats
            .iter()
            .find(|format| format.extension == "acb")
            .expect("bitmap compatibility should be public");
        assert_eq!(bitmap.current_version, 1);
        assert_eq!(bitmap.legacy_magics, vec!["ACB0"]);
        assert_eq!(
            bitmap.compatibility_rule,
            "ACB0 remains read-only compatible"
        );

        let lexical = summary
            .storage_formats
            .iter()
            .find(|format| format.extension == "aci")
            .expect("lexical index compatibility should be public");
        assert_eq!(lexical.current_version, 3);
        assert_eq!(lexical.legacy_magics, vec!["ACI0", "ACI1", "ACI2"]);
        assert_eq!(
            lexical.compatibility_rule,
            "ACI0, ACI1 and ACI2 remain read-only compatible"
        );
    }

    #[test]
    fn migration_registry_centralizes_format_and_release_versions() {
        let summary = compatibility_summary();
        assert_eq!(summary.migration_registry.current_release, "v0.2.0-beta.2");
        assert!(summary
            .migration_registry
            .downgrade_policy
            .contains("restore"));
        assert_eq!(
            summary.migration_registry.formats.len(),
            summary.storage_formats.len()
        );

        let segment = summary
            .migration_registry
            .formats
            .iter()
            .find(|format| format.kind == "segment")
            .expect("segment migration format should be registered");
        assert_eq!(segment.extension, "acs");
        assert_eq!(segment.current_magic, "ACS3");
        assert_eq!(segment.legacy_magics, vec!["ACS1", "ACS2"]);
        assert_eq!(segment.required_gate, "make storage-format-freeze-check");

        let release = summary
            .migration_registry
            .releases
            .iter()
            .find(|release| release.from == "v0.1.0-core-alpha.5")
            .expect("previous release migration path should be registered");
        assert_eq!(release.to, "v0.2.0-beta.2");
        assert!(release
            .fixture
            .as_deref()
            .unwrap_or_default()
            .contains("fixtures/migration/historical"));
        assert!(release.downgrade.contains("restore-only"));
    }
}
