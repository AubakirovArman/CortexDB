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

pub fn compatibility_summary() -> CompatibilitySummary {
    let matrix: serde_json::Value =
        serde_json::from_str(MATRIX_JSON).unwrap_or(serde_json::Value::Null);
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
        storage_formats: storage_format_specs()
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
    }
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
    }
}
