use cortex_storage::wal::{DurabilityMode, WalCodec, WalReader, WalWriter, WalWriterOptions};
use cortex_storage::StorageError;

use super::{record, record_with_payload};

#[test]
fn writer_appends_and_reader_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("writer.aclog");
    let writer = WalWriter::create(&path).unwrap();
    let ack = writer.append(record()).unwrap();
    assert_eq!(ack.durable_lsn, WalCodec::file_header_len() as u64);
    drop(writer);
    let scan = WalReader::open(&path).unwrap().scan().unwrap();
    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.records[0].record, record());
}

#[test]
fn balanced_writer_mode_appends_without_sync_requirement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("balanced.aclog");
    let writer = WalWriter::start(&path, DurabilityMode::Balanced).unwrap();
    let ack = writer.append(record_with_payload(vec![1, 2, 3])).unwrap();
    assert_eq!(ack.durable_lsn, WalCodec::file_header_len() as u64);
}

#[test]
fn bounded_writer_queue_mode_appends_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bounded.aclog");
    let writer = WalWriter::start_with_options(
        &path,
        WalWriterOptions {
            durability_mode: DurabilityMode::Balanced,
            queue_capacity: Some(1),
            max_wal_size: None,
        },
    )
    .unwrap();
    writer.append(record_with_payload(vec![1])).unwrap();
    writer.append(record_with_payload(vec![2])).unwrap();
    writer.shutdown().unwrap();

    let scan = WalReader::open(&path).unwrap().scan().unwrap();
    assert_eq!(scan.records.len(), 2);
}

#[test]
fn wal_size_based_rotation_and_archiving() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.aclog");
    let writer = WalWriter::start_with_options(
        &path,
        WalWriterOptions {
            durability_mode: DurabilityMode::Strict,
            queue_capacity: None,
            max_wal_size: Some(100),
        },
    )
    .unwrap();

    for i in 0..5 {
        writer.append(record_with_payload(vec![i])).unwrap();
    }
    writer.shutdown().unwrap();

    let mut aclog_files = vec![];
    for entry in std::fs::read_dir(dir.path()).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("db.") && name.ends_with(".aclog") {
            aclog_files.push(name);
        }
    }
    assert!(aclog_files.len() >= 2);
}

#[test]
fn balanced_writer_group_commit_accepts_parallel_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("group.aclog");
    let writer = WalWriter::start(&path, DurabilityMode::Balanced).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let handles = (0..4)
        .map(|value| {
            let writer = writer.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                writer.append(record_with_payload(vec![value])).unwrap();
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }
    writer.shutdown().unwrap();

    let scan = WalReader::open(&path).unwrap().scan().unwrap();
    assert_eq!(scan.records.len(), 4);
}

#[test]
fn writer_metrics_report_records_bytes_fsyncs_and_batches() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metrics.aclog");
    let writer = WalWriter::create(&path).unwrap();
    writer.append(record_with_payload(vec![1, 2])).unwrap();
    writer.append(record_with_payload(vec![3, 4])).unwrap();

    let metrics = writer.metrics().unwrap();
    assert_eq!(metrics.records_written, 2);
    assert!(metrics.bytes_written > 0);
    assert_eq!(metrics.fsync_count, 2);
    assert_eq!(metrics.batches_committed, 2);
}

#[test]
fn writer_start_surfaces_open_error_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let error = WalWriter::start(dir.path(), DurabilityMode::Strict).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("io error:"));
    assert!(
        message.contains("directory") || message.contains("Directory"),
        "unexpected error message: {message}"
    );
}

#[test]
fn append_after_shutdown_reports_closed_reason() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("closed-reason.aclog");
    let writer = WalWriter::create(&path).unwrap();
    writer.shutdown().unwrap();

    let error = writer.append(record()).unwrap_err();
    assert!(matches!(
        error,
        StorageError::WalWriterClosed(ref reason) if reason.contains("shutdown requested")
    ));
    assert_eq!(
        error.to_string(),
        "WAL writer is closed: shutdown requested"
    );
}
