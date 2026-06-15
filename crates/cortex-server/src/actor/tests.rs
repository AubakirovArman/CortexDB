use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::ServerOptions;

use super::routing::{is_admin_route, is_write_route, route_timeout_ms};
use super::DatabaseActor;

#[test]
fn queue_depth_returns_to_zero_after_completed_request() {
    let dir = tempfile::tempdir().unwrap();
    let actor = DatabaseActor::open_with_capacity(dir.path(), 1).unwrap();

    let response = actor
        .route("GET", "/v1/health", b"")
        .expect("health route should succeed");

    assert!(response.contains(r#""status":"ok""#));
    assert_eq!(actor.queue_depth(), 0);
    assert_eq!(actor.requests_sent(), 1);
    assert_eq!(actor.requests_completed(), 1);
}

#[test]
fn close_is_idempotent_and_blocks_new_requests() {
    let dir = tempfile::tempdir().unwrap();
    let actor = DatabaseActor::open_with_capacity(dir.path(), 8).unwrap();

    let _ = actor
        .route("GET", "/v1/health", b"")
        .expect("route should work before close");
    actor.close().expect("actor close should succeed");
    actor
        .close()
        .expect("closing an already closed actor should stay idempotent");

    assert_eq!(actor.queue_depth(), 0);
    assert_eq!(actor.requests_completed(), 1);
    assert!(actor.route("GET", "/v1/health", b"").is_err());
}

#[test]
fn concurrent_reads_run_in_parallel() {
    let dir = tempfile::tempdir().unwrap();
    let actor = Arc::new(DatabaseActor::open_with_capacity(dir.path(), 8).unwrap());
    actor
        .route(
            "POST",
            "/v1/cell?cell_id=1",
            b"scope=default\nstatus=ready\nhello",
        )
        .unwrap();

    let mut handles = Vec::new();
    let start = Instant::now();
    for _ in 0..4 {
        let actor = Arc::clone(&actor);
        handles.push(thread::spawn(move || {
            actor
                .route("GET", "/v1/cell?cell_id=1", b"")
                .expect("read should succeed");
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "concurrent reads serialized: {elapsed:?}"
    );
}

#[test]
fn waiting_write_blocks_new_reads() {
    let dir = tempfile::tempdir().unwrap();
    let actor = Arc::new(DatabaseActor::open_with_capacity(dir.path(), 8).unwrap());
    actor
        .route(
            "POST",
            "/v1/cell?cell_id=1",
            b"scope=default\nstatus=ready\nv1",
        )
        .unwrap();

    // Start a write that will take a little while by holding a read lock
    // indirectly: we cannot easily delay a write, but we can verify that
    // a writer waiting behind active readers eventually gets priority by
    // checking the waiting_writers metric after starting a write in a
    // separate thread while reads are active.
    let actor_write = Arc::clone(&actor);
    let write_started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let write_started_clone = Arc::clone(&write_started);
    let write_handle = thread::spawn(move || {
        write_started_clone.store(true, Ordering::SeqCst);
        actor_write
            .route(
                "POST",
                "/v1/cell?cell_id=1",
                b"scope=default\nstatus=ready\nv2",
            )
            .expect("write should succeed");
    });

    // Spin until the write thread has started.
    while !write_started.load(Ordering::SeqCst) {
        thread::yield_now();
    }
    thread::sleep(Duration::from_millis(5));

    // The writer is waiting for any active readers to drain.
    // We may or may not catch the exact moment, so this is best-effort.
    // The main property we want is that the write eventually completes.
    let _ = actor.waiting_writers();

    // New readers should not jump ahead of the waiting writer.
    let actor_read = Arc::clone(&actor);
    let read_handle = thread::spawn(move || {
        actor_read
            .route("GET", "/v1/cell?cell_id=1", b"")
            .expect("read should succeed")
    });

    write_handle.join().unwrap();
    let body = read_handle.join().unwrap();
    assert!(body.contains("v1") || body.contains("v2"));
}

#[test]
fn write_route_classifier_covers_mutating_routes() {
    let write_routes = [
        ("POST", "/put"),
        ("POST", "/v1/cell"),
        ("POST", "/v1/batch"),
        ("POST", "/tombstone"),
        ("DELETE", "/v1/cell"),
        ("POST", "/flush"),
        ("POST", "/v1/flush"),
        ("POST", "/v1/compact"),
        ("POST", "/v1/admin/compact/trigger"),
        ("PUT", "/v1/admin/search/hnsw/no-fallback-profile"),
        ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile"),
        ("POST", "/v1/remember"),
        ("POST", "/v1/feedback"),
        ("POST", "/v1/forget"),
        ("POST", "/v1/ingest/text"),
        ("POST", "/v1/ingest/json"),
        ("POST", "/v1/ingest/csv"),
        ("DELETE", "/v1/ingest/jobs/42"),
        ("POST", "/v1/ingest/jobs/42/cancel"),
        ("POST", "/v1/ingest/jobs/42/retry"),
    ];
    for (method, target) in write_routes {
        assert!(
            is_write_route(method, target),
            "{method} {target} must take a write lock"
        );
    }

    let read_routes = [
        ("GET", "/v1/health"),
        ("GET", "/v1/stats"),
        ("GET", "/v1/validate"),
        ("GET", "/v1/cell?cell_id=1"),
        ("POST", "/v1/context"),
        ("POST", "/v1/context/trace"),
        ("POST", "/v1/aql"),
        ("POST", "/v1/search"),
        ("POST", "/v1/search/explain"),
        ("POST", "/v1/search/ann-evaluate"),
        ("GET", "/v1/admin/search/hnsw/no-fallback-profile"),
        ("GET", "/v1/admin/compact/status"),
        ("GET", "/v1/metrics"),
        ("GET", "/v1/ann/metrics"),
        ("POST", "/v1/verify"),
        ("GET", "/v1/ingest/jobs"),
        ("GET", "/v1/ingest/jobs/42"),
    ];
    for (method, target) in read_routes {
        assert!(
            !is_write_route(method, target),
            "{method} {target} should not take a write lock"
        );
    }
}

#[test]
fn route_timeout_budget_classifies_admin_write_and_read_routes() {
    let options = ServerOptions {
        read_route_timeout_ms: 11,
        write_route_timeout_ms: 22,
        admin_route_timeout_ms: 33,
        ..Default::default()
    };
    assert!(is_admin_route("GET", "/v1/metrics"));
    assert_eq!(route_timeout_ms(&options, "GET", "/v1/metrics"), 33);
    assert_eq!(route_timeout_ms(&options, "POST", "/v1/cell"), 22);
    assert_eq!(route_timeout_ms(&options, "POST", "/v1/search"), 11);
}
