use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use cortex_core::CommitSeq;
use cortex_storage::manifest::StorageManifest;
use cortex_storage::wal::{DecodedWalRecord, WalCodec, WalReader};

use crate::checkpoint::manifest_path;
use crate::database_files::find_wal_files;
use crate::error::{EngineError, EngineResult};
use crate::operation::{
    decoded_operation_from_wal_record, decoded_write_batch_marker, DecodedWriteBatchMarker,
    WriteBatchMarker,
};

pub(crate) const WAL_ARCHIVE_DIR: &str = "wal_archive";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PitrRestoreReport {
    pub target_seq: CommitSeq,
    pub restored_seq: CommitSeq,
    pub wal_archive_files_staged: usize,
    pub wal_records_pruned: usize,
}

pub(crate) fn archive_closed_wal(
    root: &Path,
    rotated_wal: &Path,
    max_files: usize,
) -> EngineResult<()> {
    if !rotated_wal.exists() {
        return Ok(());
    }
    let archive_dir = root.join(WAL_ARCHIVE_DIR);
    fs::create_dir_all(&archive_dir)?;
    let file_name = rotated_wal.file_name().ok_or_else(|| {
        EngineError::StorageInvariant(format!(
            "invalid WAL archive path: {}",
            rotated_wal.display()
        ))
    })?;
    let target = archive_dir.join(file_name);
    fs::copy(rotated_wal, &target)?;
    sync_file_and_parent(&target)?;
    enforce_archive_retention(&archive_dir, max_files.max(1))?;
    Ok(())
}

pub(crate) fn restore_to_seq_in_place(
    root: &Path,
    target_seq: CommitSeq,
) -> EngineResult<PitrRestoreReport> {
    let staged = stage_archived_wals(root)?;
    cap_manifest_to_seq(root, target_seq)?;
    let pruned = prune_wal_records_after_seq(root, target_seq)?;
    Ok(PitrRestoreReport {
        target_seq,
        restored_seq: target_seq,
        wal_archive_files_staged: staged,
        wal_records_pruned: pruned,
    })
}

fn stage_archived_wals(root: &Path) -> EngineResult<usize> {
    let archive_dir = root.join(WAL_ARCHIVE_DIR);
    if !archive_dir.exists() {
        return Ok(0);
    }
    let mut staged = 0;
    for path in sorted_wal_files(&archive_dir)? {
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let target = root.join(file_name);
        if !target.exists() {
            fs::copy(&path, &target)?;
            sync_file_and_parent(&target)?;
            staged += 1;
        }
    }
    Ok(staged)
}

fn cap_manifest_to_seq(root: &Path, target_seq: CommitSeq) -> EngineResult<()> {
    let path = manifest_path(root);
    let mut manifest = StorageManifest::load(&path)?;
    if manifest.checkpoint_seq <= target_seq.0
        && manifest
            .live_segments
            .iter()
            .all(|segment| segment.checkpoint_seq <= target_seq.0)
    {
        return Ok(());
    }
    manifest
        .live_segments
        .retain(|segment| segment.checkpoint_seq <= target_seq.0);
    let live_ids = manifest
        .live_segments
        .iter()
        .map(|segment| segment.id)
        .collect::<std::collections::BTreeSet<_>>();
    manifest
        .segment_stats
        .retain(|stats| live_ids.contains(&stats.segment_id));
    manifest.checkpoint_seq = manifest
        .live_segments
        .iter()
        .map(|segment| segment.checkpoint_seq)
        .max()
        .unwrap_or(0);
    manifest.generation = manifest.generation.saturating_add(1);
    manifest.store(path)?;
    Ok(())
}

fn prune_wal_records_after_seq(root: &Path, target_seq: CommitSeq) -> EngineResult<usize> {
    let active = root.join("db.aclog");
    let mut pruned = 0;
    for path in find_wal_files(&active) {
        if !path.exists() {
            continue;
        }
        pruned += prune_one_wal_file(&path, target_seq)?;
    }
    Ok(pruned)
}

fn prune_one_wal_file(path: &Path, target_seq: CommitSeq) -> EngineResult<usize> {
    let scan = WalReader::scan_path(path)?;
    let mut kept = Vec::new();
    let mut pruned = 0;
    let mut index = 0;
    while index < scan.records.len() {
        let group = WalRecordGroup::read(&scan.records, index)?;
        index = group.next_index;
        match group.position(target_seq)? {
            WalGroupPosition::Keep => kept.extend(group.records.iter().copied()),
            WalGroupPosition::Prune => pruned += group.records.len(),
        }
    }
    if pruned > 0 {
        write_wal_records(path, &kept)?;
    }
    Ok(pruned)
}

struct WalRecordGroup<'a> {
    records: Vec<&'a DecodedWalRecord>,
    start_seq: CommitSeq,
    end_seq: CommitSeq,
    next_index: usize,
}

enum WalGroupPosition {
    Keep,
    Prune,
}

impl<'a> WalRecordGroup<'a> {
    fn read(records: &'a [DecodedWalRecord], start: usize) -> EngineResult<Self> {
        if let Some(marker) = decoded_write_batch_marker(&records[start])? {
            return Self::read_batch(records, start, marker);
        }
        let decoded = decoded_operation_from_wal_record(&records[start])?;
        Ok(Self {
            records: vec![&records[start]],
            start_seq: decoded.seq,
            end_seq: decoded.seq,
            next_index: start + 1,
        })
    }

    fn read_batch(
        records: &'a [DecodedWalRecord],
        start: usize,
        marker: DecodedWriteBatchMarker,
    ) -> EngineResult<Self> {
        let DecodedWriteBatchMarker::Begin(begin) = marker else {
            return Err(EngineError::StorageInvariant(
                "WAL write batch commit without begin during PITR prune".to_owned(),
            ));
        };
        let mut group = vec![&records[start]];
        let mut index = start + 1;
        while index < records.len() {
            group.push(&records[index]);
            if let Some(DecodedWriteBatchMarker::Commit(commit)) =
                decoded_write_batch_marker(&records[index])?
            {
                validate_batch_markers(begin, commit)?;
                return Ok(Self {
                    records: group,
                    start_seq: begin.start_seq,
                    end_seq: begin.end_seq,
                    next_index: index + 1,
                });
            }
            index += 1;
        }
        Err(EngineError::StorageInvariant(
            "WAL write batch begin without commit during PITR prune".to_owned(),
        ))
    }

    fn position(&self, target_seq: CommitSeq) -> EngineResult<WalGroupPosition> {
        if self.end_seq <= target_seq {
            return Ok(WalGroupPosition::Keep);
        }
        if self.start_seq > target_seq {
            return Ok(WalGroupPosition::Prune);
        }
        Err(EngineError::StorageInvariant(format!(
            "restore --to-seq {} splits atomic WAL batch {}..{}",
            target_seq.0, self.start_seq.0, self.end_seq.0
        )))
    }
}

fn validate_batch_markers(begin: WriteBatchMarker, commit: WriteBatchMarker) -> EngineResult<()> {
    if begin != commit {
        return Err(EngineError::StorageInvariant(
            "WAL write batch begin/commit marker mismatch during PITR prune".to_owned(),
        ));
    }
    Ok(())
}

fn write_wal_records(path: &Path, records: &[&DecodedWalRecord]) -> EngineResult<()> {
    let tmp = path.with_extension("aclog.tmp");
    let mut file = File::create(&tmp)?;
    file.write_all(&WalCodec::file_header())?;
    for record in records {
        let encoded = WalCodec::encode_record_at(&record.record, record.lsn)?;
        file.write_all(&encoded)?;
    }
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    sync_parent(path)?;
    Ok(())
}

fn enforce_archive_retention(archive_dir: &Path, max_files: usize) -> EngineResult<()> {
    let files = sorted_wal_files(archive_dir)?;
    if files.len() <= max_files {
        return Ok(());
    }
    let remove_count = files.len() - max_files;
    for path in files.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }
    File::open(archive_dir)?.sync_all()?;
    Ok(())
}

fn sorted_wal_files(dir: &Path) -> EngineResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_rotated_wal(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn is_rotated_wal(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with("db.") && name.ends_with(".aclog") && name != "db.aclog"
        })
}

fn sync_file_and_parent(path: &Path) -> EngineResult<()> {
    File::open(path)?.sync_all()?;
    sync_parent(path)
}

fn sync_parent(path: &Path) -> EngineResult<()> {
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}
