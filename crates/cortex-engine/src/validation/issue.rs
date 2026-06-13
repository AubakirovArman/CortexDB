use std::path::Path;

use super::StorageValidationReport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageValidationIssueKind {
    Manifest,
    Wal,
    Segment,
    BitmapIndex,
    LexicalIndex,
    VectorIndex,
    HnswGraph,
    CandidateMapping,
    ManifestReference,
}

impl StorageValidationIssueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Wal => "wal",
            Self::Segment => "segment",
            Self::BitmapIndex => "bitmap_index",
            Self::LexicalIndex => "lexical_index",
            Self::VectorIndex => "vector_index",
            Self::HnswGraph => "hnsw_graph",
            Self::CandidateMapping => "candidate_mapping",
            Self::ManifestReference => "manifest_reference",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageRecoveryAction {
    RepairWalTail,
    RebuildVectorArtifacts,
    RestoreFromBackup,
    InspectAndRepairDryRun,
}

impl StorageRecoveryAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepairWalTail => "repair_wal_tail",
            Self::RebuildVectorArtifacts => "rebuild_vector_artifacts",
            Self::RestoreFromBackup => "restore_from_backup",
            Self::InspectAndRepairDryRun => "inspect_and_repair_dry_run",
        }
    }

    pub fn command_for(self, db_path: &Path) -> String {
        let path = db_path.display();
        match self {
            Self::RepairWalTail => {
                format!("cortexdb wal-validate {path} && cortexdb repair --dry-run {path}")
            }
            Self::RebuildVectorArtifacts => {
                format!("cortexdb ann-validate {path} && cortexdb vector rebuild {path} --experimental-hnsw")
            }
            Self::RestoreFromBackup => {
                format!("cortexdb validate {path}; cortexdb restore <backup_path> <restore_path>")
            }
            Self::InspectAndRepairDryRun => format!("cortexdb repair --dry-run {path}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageValidationIssue {
    pub kind: StorageValidationIssueKind,
    pub message: String,
    pub recovery_action: StorageRecoveryAction,
    pub requires_restore: bool,
}

impl StorageValidationIssue {
    pub fn recommended_command(&self, db_path: &Path) -> String {
        self.recovery_action.command_for(db_path)
    }
}

impl StorageValidationReport {
    pub(crate) fn push_issue(
        &mut self,
        kind: StorageValidationIssueKind,
        message: impl Into<String>,
    ) {
        let message = message.into();
        self.errors.push(message.clone());
        self.issues.push(StorageValidationIssue {
            kind,
            message,
            recovery_action: recovery_action_for(kind),
            requires_restore: requires_restore(kind),
        });
    }
}

fn recovery_action_for(kind: StorageValidationIssueKind) -> StorageRecoveryAction {
    match kind {
        StorageValidationIssueKind::Wal => StorageRecoveryAction::RepairWalTail,
        StorageValidationIssueKind::VectorIndex | StorageValidationIssueKind::HnswGraph => {
            StorageRecoveryAction::RebuildVectorArtifacts
        }
        StorageValidationIssueKind::Manifest
        | StorageValidationIssueKind::Segment
        | StorageValidationIssueKind::BitmapIndex
        | StorageValidationIssueKind::LexicalIndex
        | StorageValidationIssueKind::CandidateMapping
        | StorageValidationIssueKind::ManifestReference => StorageRecoveryAction::RestoreFromBackup,
    }
}

fn requires_restore(kind: StorageValidationIssueKind) -> bool {
    matches!(
        kind,
        StorageValidationIssueKind::Manifest
            | StorageValidationIssueKind::Segment
            | StorageValidationIssueKind::BitmapIndex
            | StorageValidationIssueKind::LexicalIndex
            | StorageValidationIssueKind::CandidateMapping
            | StorageValidationIssueKind::ManifestReference
    )
}
