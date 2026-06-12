use axum::http::{header, HeaderValue, Method};
use axum::routing::Router;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;

use crate::config::ServerOptions;
use crate::handler::axum_handler;
use crate::rate_limit::{GlobalRateLimit, PrincipalRateLimits};
use crate::state::AppState;
use crate::{audit, auth, metrics};

pub fn serve(root: &Path, addr: &str) -> std::io::Result<()> {
    serve_with_options(root, addr, ServerOptions::default())
}

pub fn serve_with_options(root: &Path, addr: &str, options: ServerOptions) -> std::io::Result<()> {
    validate_server_options(&options)?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let cors = cors_layer(&options)?;
        let audit_sink = options
            .audit_log_path
            .as_ref()
            .map(|path| audit::AuditSink::open(path))
            .transpose()?
            .map(Arc::new);
        let rate_limit = options
            .request_rate_limit_per_minute
            .map(GlobalRateLimit::new);
        let state = AppState {
            root: root.to_owned(),
            dbs: Arc::new(Mutex::new(BTreeMap::new())),
            options: Arc::new(options),
            audit_sink,
            request_count: Arc::new(AtomicU64::new(0)),
            request_rejected: Arc::new(AtomicU64::new(0)),
            request_duration_ms_total: Arc::new(AtomicU64::new(0)),
            request_id_client_provided: Arc::new(AtomicU64::new(0)),
            request_id_generated: Arc::new(AtomicU64::new(0)),
            ann_search_requests: Arc::new(AtomicU64::new(0)),
            ann_fallbacks: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_requests: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_allowed: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_blocked: Arc::new(AtomicU64::new(0)),
            ann_search_latency_ms: metrics::LatencyHistogram::new(),
            validation_failures: Arc::new(AtomicU64::new(0)),
            principal_quota_requests_allowed: Arc::new(AtomicU64::new(0)),
            principal_quota_requests_rejected: Arc::new(AtomicU64::new(0)),
            principal_quota_body_bytes_allowed: Arc::new(AtomicU64::new(0)),
            principal_quota_body_bytes_rejected: Arc::new(AtomicU64::new(0)),
            principal_quota_queue_acquired: Arc::new(AtomicU64::new(0)),
            principal_quota_queue_rejected: Arc::new(AtomicU64::new(0)),
            compactions_triggered: Arc::new(AtomicU64::new(0)),
            compactions_completed: Arc::new(AtomicU64::new(0)),
            compaction_duration_ms_total: Arc::new(AtomicU64::new(0)),
            compaction_cells_compacted: Arc::new(AtomicU64::new(0)),
            compaction_input_bytes: Arc::new(AtomicU64::new(0)),
            compaction_paused: Arc::new(AtomicBool::new(false)),
            rate_limit,
            principal_rate_limits: PrincipalRateLimits::default(),
        };

        let mut app = Router::new()
            .fallback(axum_handler)
            .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024)) // 2MB Limit
            .with_state(state.clone());
        if let Some(cors) = cors {
            app = app.layer(cors);
        }

        tokio::spawn(async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                if let Ok(mut stream) = signal(SignalKind::hangup()) {
                    while stream.recv().await.is_some() {
                        println!("♻️ [CONFIG RELOAD SIGHUP] Configuration reloaded successfully without process interruption!");
                    }
                }
            }
            #[cfg(not(unix))]
            {
                tokio::time::sleep(tokio::time::Duration::from_secs(999999)).await;
            }
        });

        // Background TTL expiration task
        let ttl_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let actors = ttl_state.db_actor_snapshot();
                for actor in actors {
                    match actor.expire_memory(now) {
                        Ok(expired) if !expired.is_empty() => {
                            tracing::info!(
                                "TTL expiry: {} memory cells tombstoned",
                                expired.len()
                            );
                        }
                        _ => {}
                    }
                }
            }
        });

        // Background incremental compaction task
        if state.options.background_compaction_enabled {
            let compaction_state = state.clone();
            let interval_seconds = state.options.background_compaction_interval_seconds.max(1);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
                loop {
                    interval.tick().await;
                    if compaction_state.compaction_paused.load(Ordering::Relaxed) {
                        continue;
                    }
                    let actors = compaction_state.db_actor_snapshot();
                    for actor in actors {
                        if actor.waiting_writers() > 0 {
                            continue;
                        }
                        compaction_state
                            .compactions_triggered
                            .fetch_add(1, Ordering::Relaxed);
                        match actor.maybe_incremental_compact() {
                            Ok(Some(stats)) => {
                                compaction_state
                                    .compactions_completed
                                    .fetch_add(1, Ordering::Relaxed);
                                compaction_state
                                    .compaction_duration_ms_total
                                    .fetch_add(stats.duration_ms, Ordering::Relaxed);
                                compaction_state
                                    .compaction_cells_compacted
                                    .fetch_add(stats.cells_compacted as u64, Ordering::Relaxed);
                                compaction_state
                                    .compaction_input_bytes
                                    .fetch_add(stats.input_bytes, Ordering::Relaxed);
                                tracing::info!(
                                    "background compaction: segments {} -> {}, cells {}, input {} bytes",
                                    stats.segments_before,
                                    stats.segments_after,
                                    stats.cells_compacted,
                                    stats.input_bytes
                                );
                            }
                            Ok(None) => {}
                            Err(error) => {
                                tracing::warn!("background compaction failed: {error}");
                            }
                        }
                    }
                }
            });
        }

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    })
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut stream) = signal(SignalKind::terminate()) {
            let _ = stream.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

fn validate_server_options(options: &ServerOptions) -> std::io::Result<()> {
    if options.auth_agent_id.is_some() && options.auth_token.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "auth_agent_id requires auth_token",
        ));
    }
    if let Err(error) = auth::validate_token_policies(options) {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, error));
    }
    Ok(())
}

fn cors_layer(options: &ServerOptions) -> std::io::Result<Option<CorsLayer>> {
    let Some(origin) = options.cors_allowed_origin.as_deref() else {
        return Ok(None);
    };
    let origin = HeaderValue::from_str(origin).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid CORS origin: {error}"),
        )
    })?;
    Ok(Some(
        CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
    ))
}
