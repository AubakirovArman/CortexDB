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
