function buildSloDashboard({ health, compatibility, stats, validation, metrics, backup, incidents }) {
    const latencyBudgetMs = 1000;
    const backupFreshnessBudgetSeconds = 86400;
    const rejected = metrics.request_rejected + metrics.quota_rejected;
    const errorsUsed = rejected + metrics.validation_failures + incidents.length;
    const meanLatency = metrics.mean_latency_ms;
    const backupAge = metrics.backup_latest_age_seconds;
    const backupKnown = Number.isFinite(backupAge) && backupAge >= 0;

    return {
        schema_version: "dashboard_slo.v1",
        availability: {
            ok: !!health.ok && !!compatibility.ok,
            signal: health.ok && compatibility.ok ? "available" : "attention",
            health: health.message || "not checked",
            compatibility: compatibility.message || "not checked",
        },
        latency: {
            ok: metrics.available && meanLatency !== null ? meanLatency <= latencyBudgetMs : null,
            mean_ms: meanLatency === null ? null : Number(meanLatency.toFixed(2)),
            budget_ms: latencyBudgetMs,
            request_count: metrics.request_count,
            signal: !metrics.available
                ? metrics.message
                : (meanLatency === null ? "no traffic yet" : `${meanLatency.toFixed(2)} ms mean`),
        },
        backup_freshness: {
            ok: backupKnown ? backupAge <= backupFreshnessBudgetSeconds : null,
            age_seconds: backupKnown ? backupAge : null,
            budget_seconds: backupFreshnessBudgetSeconds,
            signal: backupKnown ? `${backupAge}s since latest backup` : backup.message,
            evidence_gate: backup.evidence_gate,
        },
        validation_status: {
            ok: !!validation.ok,
            signal: validation.ok ? "valid" : validation.message || "not checked",
            errors: validation.errors || [],
            manifest_ok: validation.manifest_ok,
            wal_ok: validation.wal_ok,
        },
        error_budget: {
            ok: metrics.available ? errorsUsed === 0 : null,
            errors_used: errorsUsed,
            request_rejected: metrics.request_rejected,
            quota_rejected: metrics.quota_rejected,
            validation_failures: metrics.validation_failures,
            visible_incidents: incidents.length,
            signal: !metrics.available ? metrics.message : (errorsUsed === 0 ? "clean" : `${errorsUsed} issue(s) used`),
        },
        operator_actions: [
            "Refresh Status before maintenance.",
            "Run make load-smoke-check and make performance-trend-check for release latency evidence.",
            "Run make backup-restore-production-pack-check for backup freshness evidence.",
            "Run validate before trusting checkpoint, compact, or backup workflows.",
        ],
        stats_context: {
            current_seq: stats.current_seq,
            checkpoint_seq: stats.checkpoint_seq,
            live_segments: stats.live_segments,
            wal_size_bytes: stats.wal_size_bytes,
        },
    };
}

function buildBackupRestoreView({ metrics, validation, backup }) {
    const backupAge = metrics.backup_latest_age_seconds;
    const backupKnown = Number.isFinite(backupAge) && backupAge >= 0;
    const rpoBudgetSeconds = 86400;
    const rtoEvidenceGate = "make backup-restore-production-pack-check";
    const validationOk = validation.ok === true;
    const adminAvailable = backup.available === true;
    return {
        schema_version: "dashboard_backup_restore.v1",
        latest_backup: {
            available: backupKnown,
            age_seconds: backupKnown ? backupAge : null,
            status: backupKnown ? (backupAge <= rpoBudgetSeconds ? "within_rpo" : "stale") : "unknown",
            evidence_source: "cortexdb_backup_latest_age_seconds",
        },
        restore_drill: {
            status: adminAvailable && validationOk ? "operator_gate_ready" : "operator_gate_required",
            evidence_gate: rtoEvidenceGate,
            command: "cortexdb backup-drill",
        },
        offsite: {
            status: adminAvailable && validationOk ? "operator_gate_ready" : "operator_gate_required",
            evidence_gate: "make backup-offsite-check",
            command: "cortexdb backup-offsite-stage",
        },
        rpo_rto: {
            rpo_budget_seconds: rpoBudgetSeconds,
            rpo_status: backupKnown ? (backupAge <= rpoBudgetSeconds ? "within_budget" : "stale") : "unknown",
            rto_status: "drill_required_for_release",
            rto_evidence_gate: rtoEvidenceGate,
        },
        note: "Dashboard shows backup/restore posture; backup, restore, and offsite publication remain operator CLI workflows.",
    };
}

function backupPosture(results) {
    const validation = summarizeValidationResult(results);
    return {
        available: canUse(ACCESS_ADMIN),
        browser_runs_backup: false,
        mode: "operator_cli",
        evidence_gate: "make backup-restore-production-pack-check",
        commands: [
            "cortexdb backup",
            "cortexdb backup-drill",
            "cortexdb backup-encrypted",
            "cortexdb backup-offsite-stage",
        ],
        validation_ok: validation.ok,
        message: canUse(ACCESS_ADMIN)
            ? "Backups are operator CLI actions; validate storage here before trusting backup posture."
            : "Admin token required to inspect storage before backup.",
    };
}
