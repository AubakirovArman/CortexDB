use cortex_engine::{Database, IngestionJobStatus, IngestionProgress, IngestionProgressTracker};

#[test]
fn ingestion_job_durable_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let progress = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(42),
        label: "test-job".to_owned(),
        status: IngestionJobStatus::Running,
        total_items: Some(100),
        completed_items: 50,
        failed_items: 2,
        last_cell_id: Some(cortex_core::CellId(7)),
        message: None,
        retry_count: 1,
        max_retries: 5,
    };

    db.save_ingestion_job(&progress).unwrap();

    let loaded = db.load_ingestion_job(42).unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.job_id.0, 42);
    assert_eq!(loaded.label, "test-job");
    assert_eq!(loaded.status, IngestionJobStatus::Running);
    assert_eq!(loaded.total_items, Some(100));
    assert_eq!(loaded.completed_items, 50);
    assert_eq!(loaded.failed_items, 2);
    assert_eq!(loaded.last_cell_id, Some(cortex_core::CellId(7)));
    assert_eq!(loaded.retry_count, 1);
    assert_eq!(loaded.max_retries, 5);
}

#[test]
fn ingestion_job_load_missing_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let loaded = db.load_ingestion_job(999).unwrap();
    assert!(loaded.is_none());
}

#[test]
fn ingestion_job_list_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let job_a = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(1),
        label: "job-a".to_owned(),
        status: IngestionJobStatus::Completed,
        total_items: Some(10),
        completed_items: 10,
        failed_items: 0,
        last_cell_id: Some(cortex_core::CellId(1)),
        message: None,
        retry_count: 0,
        max_retries: 3,
    };
    let job_b = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(2),
        label: "job-b".to_owned(),
        status: IngestionJobStatus::Failed,
        total_items: Some(5),
        completed_items: 3,
        failed_items: 2,
        last_cell_id: Some(cortex_core::CellId(2)),
        message: Some("parse error".to_owned()),
        retry_count: 2,
        max_retries: 3,
    };

    db.save_ingestion_job(&job_a).unwrap();
    db.save_ingestion_job(&job_b).unwrap();

    let list = db.list_ingestion_jobs().unwrap();
    assert_eq!(list.len(), 2);

    let labels: Vec<String> = list.iter().map(|j| j.label.clone()).collect();
    assert!(labels.contains(&"job-a".to_owned()));
    assert!(labels.contains(&"job-b".to_owned()));
}

#[test]
fn ingestion_progress_tracker_lifecycle() {
    let mut tracker = IngestionProgressTracker::default();

    let job_id = tracker.start("csv-import", Some(100)).unwrap();
    assert_eq!(
        tracker.get(job_id).unwrap().status,
        IngestionJobStatus::Running
    );

    tracker.record_cell(job_id, cortex_core::CellId(1)).unwrap();
    assert_eq!(tracker.get(job_id).unwrap().completed_items, 1);

    tracker.finish(job_id).unwrap();
    assert_eq!(
        tracker.get(job_id).unwrap().status,
        IngestionJobStatus::Completed
    );

    let job_id_2 = tracker.start("json-import", Some(50)).unwrap();
    tracker.fail(job_id_2, "invalid json").unwrap();
    assert_eq!(
        tracker.get(job_id_2).unwrap().status,
        IngestionJobStatus::Failed
    );
    assert_eq!(
        tracker.get(job_id_2).unwrap().message,
        Some("invalid json".to_owned())
    );
}

#[test]
fn ingestion_job_tracker_cancel_running() {
    let mut tracker = IngestionProgressTracker::default();
    let job_id = tracker.start("cancel-test", Some(10)).unwrap();

    tracker.cancel(job_id).unwrap();
    assert_eq!(
        tracker.get(job_id).unwrap().status,
        IngestionJobStatus::Cancelled
    );
}

#[test]
fn ingestion_job_tracker_cancel_completed_fails() {
    let mut tracker = IngestionProgressTracker::default();
    let job_id = tracker.start("cancel-test", Some(10)).unwrap();
    tracker.finish(job_id).unwrap();

    assert!(tracker.cancel(job_id).is_err());
}

#[test]
fn ingestion_job_tracker_retry_failed() {
    let mut tracker = IngestionProgressTracker::default();
    let job_id = tracker.start("retry-test", Some(10)).unwrap();
    tracker.fail(job_id, "network error").unwrap();
    assert_eq!(tracker.get(job_id).unwrap().retry_count, 0);

    tracker.retry(job_id).unwrap();
    let progress = tracker.get(job_id).unwrap();
    assert_eq!(progress.status, IngestionJobStatus::Queued);
    assert_eq!(progress.retry_count, 1);
    assert_eq!(progress.message, None);
}

#[test]
fn ingestion_job_tracker_retry_exceeds_max() {
    let mut tracker = IngestionProgressTracker::default();
    let job_id = tracker.start("retry-test", Some(10)).unwrap();
    tracker.fail(job_id, "error").unwrap();

    tracker.retry(job_id).unwrap();
    tracker.fail(job_id, "error again").unwrap();
    tracker.retry(job_id).unwrap();
    tracker.fail(job_id, "error third").unwrap();
    tracker.retry(job_id).unwrap();
    tracker.fail(job_id, "error fourth").unwrap();

    assert!(tracker.retry(job_id).is_err());
}

#[test]
fn ingestion_job_tracker_retry_non_failed_fails() {
    let mut tracker = IngestionProgressTracker::default();
    let job_id = tracker.start("retry-test", Some(10)).unwrap();

    assert!(tracker.retry(job_id).is_err());
    tracker.finish(job_id).unwrap();
    assert!(tracker.retry(job_id).is_err());
}

#[test]
fn ingestion_job_database_cancel_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let progress = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(7),
        label: "cancel-me".to_owned(),
        status: IngestionJobStatus::Running,
        total_items: Some(5),
        completed_items: 2,
        failed_items: 0,
        last_cell_id: Some(cortex_core::CellId(3)),
        message: None,
        retry_count: 0,
        max_retries: 3,
    };
    db.save_ingestion_job(&progress).unwrap();

    let cancelled = db.cancel_ingestion_job(7).unwrap();
    assert_eq!(cancelled.status, IngestionJobStatus::Cancelled);

    let loaded = db.load_ingestion_job(7).unwrap().unwrap();
    assert_eq!(loaded.status, IngestionJobStatus::Cancelled);

    let deleted = db.delete_ingestion_job(7).unwrap();
    assert!(deleted);
    assert!(db.load_ingestion_job(7).unwrap().is_none());
}

#[test]
fn ingestion_job_database_retry_persists() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let progress = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(8),
        label: "retry-me".to_owned(),
        status: IngestionJobStatus::Failed,
        total_items: Some(5),
        completed_items: 0,
        failed_items: 1,
        last_cell_id: None,
        message: Some("boom".to_owned()),
        retry_count: 0,
        max_retries: 3,
    };
    db.save_ingestion_job(&progress).unwrap();

    let retried = db.retry_ingestion_job(8).unwrap();
    assert_eq!(retried.status, IngestionJobStatus::Queued);
    assert_eq!(retried.retry_count, 1);
    assert_eq!(retried.message, None);

    let loaded = db.load_ingestion_job(8).unwrap().unwrap();
    assert_eq!(loaded.status, IngestionJobStatus::Queued);
    assert_eq!(loaded.retry_count, 1);
}

#[test]
fn ingestion_job_database_delete_missing_returns_false() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let deleted = db.delete_ingestion_job(999).unwrap();
    assert!(!deleted);
}

#[test]
fn ingestion_job_database_cancel_completed_fails() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let progress = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(9),
        label: "completed".to_owned(),
        status: IngestionJobStatus::Completed,
        total_items: Some(1),
        completed_items: 1,
        failed_items: 0,
        last_cell_id: Some(cortex_core::CellId(1)),
        message: None,
        retry_count: 0,
        max_retries: 3,
    };
    db.save_ingestion_job(&progress).unwrap();

    assert!(db.cancel_ingestion_job(9).is_err());
}

#[test]
fn ingestion_job_database_retry_non_failed_fails() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let progress = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(10),
        label: "running".to_owned(),
        status: IngestionJobStatus::Running,
        total_items: Some(1),
        completed_items: 0,
        failed_items: 0,
        last_cell_id: None,
        message: None,
        retry_count: 0,
        max_retries: 3,
    };
    db.save_ingestion_job(&progress).unwrap();

    assert!(db.retry_ingestion_job(10).is_err());
}

#[test]
fn ingestion_job_database_retry_max_retries_exceeded() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let progress = IngestionProgress {
        job_id: cortex_engine::IngestionJobId(11),
        label: "maxed".to_owned(),
        status: IngestionJobStatus::Failed,
        total_items: Some(1),
        completed_items: 0,
        failed_items: 1,
        last_cell_id: None,
        message: Some("boom".to_owned()),
        retry_count: 3,
        max_retries: 3,
    };
    db.save_ingestion_job(&progress).unwrap();

    assert!(db.retry_ingestion_job(11).is_err());
}
