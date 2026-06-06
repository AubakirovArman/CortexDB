use cortex_engine::{
    Database, DatabaseOptions, EngineError, EngineErrorCode, IngestionBackpressurePolicy,
    IngestionBackpressureRequest, IngestionJobId, IngestionJobStatus, IngestionProgress,
};

#[test]
fn ingestion_backpressure_rejects_input_over_memory_limit() {
    let dir = tempfile::tempdir().unwrap();
    let db = database_with_policy(
        IngestionBackpressurePolicy {
            max_input_bytes: 4,
            ..IngestionBackpressurePolicy::default()
        },
        dir.path(),
    );

    let error = db
        .check_ingestion_backpressure(IngestionBackpressureRequest {
            input_bytes: 5,
            total_items: None,
        })
        .unwrap_err();

    assert_eq!(error.code(), EngineErrorCode::PayloadTooLarge);
    assert!(matches!(error, EngineError::IngestionInputTooLarge { .. }));
}

#[test]
fn ingestion_backpressure_rejects_item_limit() {
    let dir = tempfile::tempdir().unwrap();
    let db = database_with_policy(
        IngestionBackpressurePolicy {
            max_total_items: 2,
            ..IngestionBackpressurePolicy::default()
        },
        dir.path(),
    );

    let error = db
        .check_ingestion_backpressure(IngestionBackpressureRequest {
            input_bytes: 1,
            total_items: Some(3),
        })
        .unwrap_err();

    assert_eq!(error.code(), EngineErrorCode::BadRequest);
    assert!(matches!(
        error,
        EngineError::IngestionItemLimitExceeded { .. }
    ));
}

#[test]
fn ingestion_backpressure_rejects_queue_limit() {
    let dir = tempfile::tempdir().unwrap();
    let db = database_with_policy(
        IngestionBackpressurePolicy {
            max_queued_jobs: 1,
            ..IngestionBackpressurePolicy::default()
        },
        dir.path(),
    );
    db.save_ingestion_job(&job(21, IngestionJobStatus::Queued))
        .unwrap();

    let error = db
        .check_ingestion_backpressure(IngestionBackpressureRequest {
            input_bytes: 1,
            total_items: Some(1),
        })
        .unwrap_err();

    assert_eq!(error.code(), EngineErrorCode::DatabaseBusy);
    assert!(matches!(error, EngineError::IngestionBackpressure { .. }));
}

#[test]
fn ingestion_backpressure_rejects_rate_limit() {
    let dir = tempfile::tempdir().unwrap();
    let db = database_with_policy(
        IngestionBackpressurePolicy {
            max_requests_per_window: 1,
            rate_window_seconds: 3600,
            ..IngestionBackpressurePolicy::default()
        },
        dir.path(),
    );
    let request = IngestionBackpressureRequest {
        input_bytes: 1,
        total_items: Some(1),
    };

    db.check_ingestion_backpressure(request).unwrap();
    let error = db.check_ingestion_backpressure(request).unwrap_err();

    assert_eq!(error.code(), EngineErrorCode::RateLimited);
    assert!(matches!(error, EngineError::IngestionRateLimited { .. }));
}

#[test]
fn ingestion_cancellation_guard_rejects_cancelled_job() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    db.save_ingestion_job(&job(22, IngestionJobStatus::Cancelled))
        .unwrap();

    let error = db
        .ensure_ingestion_job_not_cancelled(IngestionJobId(22))
        .unwrap_err();

    assert_eq!(error.code(), EngineErrorCode::BadRequest);
    assert!(matches!(error, EngineError::IngestionCancelled(22)));
}

fn database_with_policy(policy: IngestionBackpressurePolicy, path: &std::path::Path) -> Database {
    Database::open_with_options(
        path,
        DatabaseOptions {
            ingestion_backpressure: policy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap()
}

fn job(id: u64, status: IngestionJobStatus) -> IngestionProgress {
    IngestionProgress {
        job_id: IngestionJobId(id),
        label: format!("test-job-{id}"),
        status,
        total_items: Some(10),
        completed_items: 0,
        failed_items: 0,
        last_cell_id: None,
        message: None,
        retry_count: 0,
        max_retries: 3,
    }
}
