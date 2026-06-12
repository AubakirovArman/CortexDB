backup-drill-check:
	scripts/backup_drill_check.sh "$(BACKUP_DRILL_ROOT)" "$(BACKUP_DRILL_REPORT)" "$(BACKUP_DRILL_KEEP_LATEST)" "$(BACKUP_DRILL_PREFIX)"

backup-offsite-check:
	scripts/backup_offsite_check.sh "$(BACKUP_OFFSITE_ROOT)" "$(BACKUP_OFFSITE_REPORT)" "$(BACKUP_OFFSITE_ID)"

backup-rpo-rto-profile-check:
	python3 scripts/backup_rpo_rto_profiles.py --root "$(BACKUP_RPO_RTO_ROOT)" --report "$(BACKUP_RPO_RTO_REPORT)"

encrypted-backup-check:
	cargo test -p cortex-engine encrypted_backup
	cargo test -p cortex-cli backup_encrypted_and_restore_encrypted_commands_roundtrip_database
	python3 scripts/encrypted_backup_check.py --root "$(ENCRYPTED_BACKUP_ROOT)" --report "$(ENCRYPTED_BACKUP_REPORT)"

encrypted-backup-rotation-check:
	python3 scripts/encrypted_backup_rotation_check.py --root "$(ENCRYPTED_BACKUP_ROTATION_ROOT)" --report "$(ENCRYPTED_BACKUP_ROTATION_REPORT)"

backup-restore-production-pack-check:
	$(MAKE) backup-drill-check
	$(MAKE) backup-offsite-check
	$(MAKE) backup-rpo-rto-profile-check
	$(MAKE) encrypted-backup-check
	$(MAKE) encrypted-backup-rotation-check
	python3 scripts/backup_restore_production_pack.py --backup-drill-report "$(BACKUP_DRILL_REPORT)" --backup-offsite-report "$(BACKUP_OFFSITE_REPORT)" --rpo-rto-profile-report "$(BACKUP_RPO_RTO_REPORT)" --encrypted-backup-report "$(ENCRYPTED_BACKUP_REPORT)" --encrypted-backup-rotation-report "$(ENCRYPTED_BACKUP_ROTATION_REPORT)" --output "$(BACKUP_RESTORE_PACK_REPORT)"

crash-fault-check:
	scripts/crash_fault_check.sh "$(CRASH_FAULT_ROOT)" "$(CRASH_FAULT_REPORT)"

chaos-restart-check:
	python3 scripts/chaos_restart_check.py --root "$(CHAOS_RESTART_ROOT)" --report "$(CHAOS_RESTART_REPORT)" --seed "$(CHAOS_RESTART_SEED)" --steps "$(CHAOS_RESTART_STEPS)"

chaos-scenario-map-check:
	python3 scripts/chaos_scenario_map.py --output "$(CHAOS_SCENARIO_MAP_REPORT)"

graceful-shutdown-check:
	python3 scripts/graceful_shutdown_check.py --root "$(GRACEFUL_SHUTDOWN_ROOT)" --report "$(GRACEFUL_SHUTDOWN_REPORT)" --requests "$(GRACEFUL_SHUTDOWN_REQUESTS)" --shutdown-delay-ms "$(GRACEFUL_SHUTDOWN_DELAY_MS)" --max-shutdown-ms "$(GRACEFUL_SHUTDOWN_MAX_MS)" --min-acked "$(GRACEFUL_SHUTDOWN_MIN_ACKED)"

storage-soak-check:
	python3 scripts/storage_soak_check.py --root "$(STORAGE_SOAK_ROOT)" --report "$(STORAGE_SOAK_REPORT)" --cycles "$(STORAGE_SOAK_CYCLES)" --cells-per-cycle "$(STORAGE_SOAK_CELLS_PER_CYCLE)" --kill-delay-ms "$(STORAGE_SOAK_KILL_DELAY_MS)"

storage-soak-history-check: storage-soak-check
	python3 scripts/storage_soak_history_check.py --soak-report "$(STORAGE_SOAK_REPORT)" --history-jsonl "$(STORAGE_SOAK_HISTORY_FILE)" --output "$(STORAGE_SOAK_HISTORY_REPORT)" --min-runs "$(STORAGE_SOAK_HISTORY_MIN_RUNS)" --min-duration-hours "$(STORAGE_SOAK_HISTORY_MIN_HOURS)"

storage-soak-24h-campaign:
	python3 scripts/storage_soak_campaign.py --target-hours "$(STORAGE_SOAK_CAMPAIGN_TARGET_HOURS)" --max-runs "$(STORAGE_SOAK_CAMPAIGN_MAX_RUNS)" --cycles "$(STORAGE_SOAK_CAMPAIGN_CYCLES)" --cells-per-cycle "$(STORAGE_SOAK_CAMPAIGN_CELLS_PER_CYCLE)" --kill-delay-ms "$(STORAGE_SOAK_KILL_DELAY_MS)" --soak-root "$(STORAGE_SOAK_ROOT)" --soak-report "$(STORAGE_SOAK_REPORT)" --history-jsonl "$(STORAGE_SOAK_HISTORY_FILE)" --history-report "$(STORAGE_SOAK_HISTORY_REPORT)" --campaign-report "$(STORAGE_SOAK_CAMPAIGN_REPORT)"

storage-soak-campaign-status:
	python3 scripts/storage_soak_campaign_status.py --format "$(STORAGE_SOAK_CAMPAIGN_STATUS_FORMAT)"

storage-soak-campaign-watchdog:
	python3 scripts/storage_soak_campaign_status.py --require-active --max-stale-minutes "30"

storage-soak-campaign-status-check:
	python3 scripts/storage_soak_campaign_status_check.py

storage-soak-24h-evidence-check:
	python3 scripts/storage_soak_epic_finalize.py --dry-run

storage-soak-72h-start:
	python3 scripts/storage_soak_campaign_start.py --pid-file "$(STORAGE_SOAK_V2_PID_FILE)" --log-file "$(STORAGE_SOAK_V2_LOG_FILE)" -- $(MAKE) storage-soak-72h-campaign

storage-soak-72h-campaign:
	python3 scripts/storage_soak_campaign.py --target-hours "$(STORAGE_SOAK_V2_TARGET_HOURS)" --max-runs "$(STORAGE_SOAK_V2_MAX_RUNS)" --cycles "$(STORAGE_SOAK_V2_CYCLES)" --cells-per-cycle "$(STORAGE_SOAK_V2_CELLS_PER_CYCLE)" --kill-delay-ms "$(STORAGE_SOAK_KILL_DELAY_MS)" --soak-root "$(STORAGE_SOAK_V2_ROOT)" --soak-report "$(STORAGE_SOAK_V2_REPORT)" --history-jsonl "$(STORAGE_SOAK_V2_HISTORY_FILE)" --history-report "$(STORAGE_SOAK_V2_HISTORY_REPORT)" --campaign-report "$(STORAGE_SOAK_V2_CAMPAIGN_REPORT)"

storage-soak-72h-status:
	python3 scripts/storage_soak_campaign_status.py --pid-file "$(STORAGE_SOAK_V2_PID_FILE)" --campaign "$(STORAGE_SOAK_V2_CAMPAIGN_REPORT)" --history "$(STORAGE_SOAK_V2_HISTORY_REPORT)" --soak-root "$(STORAGE_SOAK_V2_ROOT)" --target-hours "$(STORAGE_SOAK_V2_TARGET_HOURS)" --format "$(STORAGE_SOAK_CAMPAIGN_STATUS_FORMAT)"

storage-soak-72h-status-report:
	python3 scripts/storage_soak_campaign_status.py --pid-file "$(STORAGE_SOAK_V2_PID_FILE)" --campaign "$(STORAGE_SOAK_V2_CAMPAIGN_REPORT)" --history "$(STORAGE_SOAK_V2_HISTORY_REPORT)" --soak-root "$(STORAGE_SOAK_V2_ROOT)" --target-hours "$(STORAGE_SOAK_V2_TARGET_HOURS)" --output "$(STORAGE_SOAK_V2_STATUS_REPORT)" --format json

storage-soak-72h-watchdog:
	python3 scripts/storage_soak_campaign_status.py --pid-file "$(STORAGE_SOAK_V2_PID_FILE)" --campaign "$(STORAGE_SOAK_V2_CAMPAIGN_REPORT)" --history "$(STORAGE_SOAK_V2_HISTORY_REPORT)" --soak-root "$(STORAGE_SOAK_V2_ROOT)" --target-hours "$(STORAGE_SOAK_V2_TARGET_HOURS)" --require-active --output "$(STORAGE_SOAK_V2_STATUS_REPORT)" --format text

storage-soak-72h-evidence-check:
	python3 scripts/storage_soak_v2_gate.py --report "$(STORAGE_SOAK_V2_HISTORY_REPORT)" --output "$(STORAGE_SOAK_V2_GATE_REPORT)" --min-hours "$(STORAGE_SOAK_V2_TARGET_HOURS)" --min-avg-cells-per-cycle "$(STORAGE_SOAK_V2_CELLS_PER_CYCLE)" --min-throughput-ratio "$(STORAGE_SOAK_V2_MIN_THROUGHPUT_RATIO)" --max-space-amplification-q16 "$(STORAGE_SOAK_V2_MAX_SPACE_AMPLIFICATION_Q16)" --max-write-amplification-q16 "$(STORAGE_SOAK_V2_MAX_WRITE_AMPLIFICATION_Q16)" --max-compaction-pressure-q16 "$(STORAGE_SOAK_V2_MAX_COMPACTION_PRESSURE_Q16)"

storage-soak-epic-finalize:
	python3 scripts/storage_soak_epic_finalize.py

next-60-epics-audit:
	python3 scripts/next_60_epics_audit.py

next-60-epics-completion-check:
	python3 scripts/next_60_epics_audit.py --require-complete --output "target/next-60-epics-audit/completion_report.json"

replication-partition-check:
	python3 scripts/replication_partition_check.py --root "$(REPLICATION_PARTITION_ROOT)" --report "$(REPLICATION_PARTITION_REPORT)"

replication-lifecycle-check:
	python3 scripts/replication_lifecycle_check.py --root "$(REPLICATION_LIFECYCLE_ROOT)" --report "$(REPLICATION_LIFECYCLE_REPORT)"

production-evidence-sweep:
	scripts/production_evidence_sweep.sh "$(PRODUCTION_EVIDENCE_ROOT)" "$(PRODUCTION_EVIDENCE_REPORT)"

