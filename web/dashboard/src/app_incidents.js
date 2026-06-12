function buildIncidentTimeline({ results, incidents, validation, metrics, backup }) {
    const timeline = [];
    const add = (category, severity, label, message, action, source = "dashboard_status") => {
        timeline.push({ category, severity, label, message, action, source });
    };

    add(
        "audit_event",
        "info",
        "Audit readiness",
        "Audit readiness is checked from the Overview audit panel.",
        "Run the audit readiness action before incident review.",
        "dashboard_audit_readiness",
    );

    for (const incident of incidents) {
        const status = Number(incident.status || 0);
        if (status === 429 || incident.code === "rate_limited") {
            add(
                "rate_limit_event",
                "warn",
                incident.label,
                incident.message,
                "Wait for the rate window to reset or review per-token quotas.",
                "request_error",
            );
        }
    }

    if (validation.available && !validation.ok) {
        add(
            "storage_event",
            "critical",
            "Storage validation",
            validation.message || "Storage validation reported errors.",
            "Run cortexdb validate, then repair or restore before trusting writes.",
            "validation",
        );
    } else if (!validation.available && canUse(ACCESS_ADMIN)) {
        add(
            "storage_event",
            "warn",
            "Storage validation",
            "Storage validation did not run.",
            "Retry validation before backup, restore, or release evidence.",
            "validation",
        );
    }

    if (backup.validation_ok === false) {
        add(
            "backup_event",
            "critical",
            "Backup posture",
            "Backup posture is blocked by failed storage validation.",
            "Fix validation failures before backup or restore drills.",
            "backup_posture",
        );
    } else if (!backup.available) {
        add(
            "backup_event",
            "warn",
            "Backup posture",
            "Backup posture requires an admin token and operator CLI evidence.",
            "Use an admin token and run make backup-restore-production-pack-check.",
            "backup_posture",
        );
    } else {
        add(
            "backup_event",
            "info",
            "Backup posture",
            "Browser view is read-only; backup evidence is produced by operator CLI gates.",
            "Run make backup-restore-production-pack-check for release evidence.",
            "backup_posture",
        );
    }

    if (metrics.available && !metrics.ok) {
        add(
            "rate_limit_event",
            "warn",
            "Metrics",
            metrics.message || "Metrics endpoint was not reachable.",
            "Check /v1/metrics before relying on quota and request counters.",
            "metrics",
        );
    }

    const requestError = lastRequestIssue;
    if (requestError) {
        add(
            requestError.status === 429 || requestError.code === "rate_limited" ? "rate_limit_event" : "audit_event",
            requestError.status === 429 ? "warn" : "info",
            requestError.label || "Last request",
            requestError.message || "Last request reported an issue.",
            "Review the typed request issue card and retry after fixing the cause.",
            "last_request_error",
        );
    }

    return timeline;
}

function buildIncidentView({ incidents, validation, metrics, backup }) {
    const requestErrors = incidents.filter((incident) => {
        const status = Number(incident.status || 0);
        return status !== 429 && incident.code !== "rate_limited";
    });
    const rateLimitEvents = incidents.filter((incident) => {
        const status = Number(incident.status || 0);
        return status === 429 || incident.code === "rate_limited";
    });
    const quotaRejected = metrics.quota_rejected || 0;
    const queueDepth = Number(metrics.actor_queue_depth || 0);
    const queueCapacity = Number(metrics.actor_queue_capacity || 0);
    const queueRatio = queueCapacity > 0 ? queueDepth / queueCapacity : 0;
    const backupAge = metrics.backup_latest_age_seconds;
    const backupKnown = Number.isFinite(backupAge) && backupAge >= 0;
    const backupStale = backupKnown && backupAge > 86400;
    const storageWarnings = [];
    const backupFailures = [];

    if (validation.available && !validation.ok) {
        storageWarnings.push({
            code: "storage_validation_failed",
            message: validation.message || "Storage validation reported errors.",
            evidence: "cortexdb validate",
        });
    } else if (!validation.available && canUse(ACCESS_ADMIN)) {
        storageWarnings.push({
            code: "storage_validation_missing",
            message: "Storage validation did not run.",
            evidence: "GET /v1/validate",
        });
    }

    if (backup.validation_ok === false) {
        backupFailures.push({
            code: "backup_blocked_by_validation",
            message: "Backup posture is blocked by failed storage validation.",
            evidence: "make backup-restore-production-pack-check",
        });
    }
    if (backupStale) {
        backupFailures.push({
            code: "backup_stale",
            message: `Latest backup evidence is ${backupAge}s old.`,
            evidence: "cortexdb_backup_latest_age_seconds",
        });
    }
    if (!backup.available) {
        backupFailures.push({
            code: "backup_admin_required",
            message: "Backup posture needs admin access and operator CLI evidence.",
            evidence: "make backup-restore-production-pack-check",
        });
    }

    return {
        schema_version: "dashboard_incident_view.v1",
        errors: {
            status: requestErrors.length ? "attention" : "clear",
            count: requestErrors.length,
            events: requestErrors,
        },
        rate_limits: {
            status: rateLimitEvents.length || quotaRejected ? "attention" : (metrics.available ? "clear" : "unknown"),
            request_rejected: metrics.request_rejected,
            quota_rejected: quotaRejected,
            principal_quota_requests_rejected: metrics.principal_quota_requests_rejected,
            principal_quota_body_bytes_rejected: metrics.principal_quota_body_bytes_rejected,
            principal_quota_queue_rejected: metrics.principal_quota_queue_rejected,
            events: rateLimitEvents,
        },
        actor_busy: {
            status: metrics.principal_quota_queue_rejected > 0 || (queueCapacity > 0 && queueDepth >= queueCapacity)
                ? "busy"
                : (queueRatio >= 0.8 ? "near_capacity" : (metrics.available ? "clear" : "unknown")),
            queue_depth: queueDepth,
            queue_capacity: queueCapacity,
            queue_ratio: Number(queueRatio.toFixed(2)),
            evidence_source: "cortexdb_actor_queue_depth",
        },
        storage_warnings: {
            status: storageWarnings.length ? "attention" : "clear",
            count: storageWarnings.length,
            events: storageWarnings,
        },
        backup_failures: {
            status: backupFailures.length ? "attention" : "clear",
            count: backupFailures.length,
            events: backupFailures,
        },
        operator_actions: [
            "Check Request issue for the last failed API call.",
            "Use /v1/metrics for quota and actor queue pressure.",
            "Run cortexdb validate before checkpoint, compact, backup, or restore work.",
            "Run make backup-restore-production-pack-check when backup evidence is stale or unknown.",
        ],
    };
}
