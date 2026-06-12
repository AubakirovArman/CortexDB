# CortexDB launchd Service

Status: local single-node macOS operator example.

This service starts the blocking single-node CortexDB core through the async
HTTP server. It is not a managed-cloud, distributed, or multi-node deployment
recipe.

## Layout

Suggested Homebrew-style paths:

```text
/usr/local/bin/cortex-server
/usr/local/var/cortexdb
/usr/local/etc/cortexdb/auth.tokens
/usr/local/var/log/cortexdb
```

Create the runtime directories:

```bash
sudo install -d -m 0750 /usr/local/var/cortexdb
sudo install -d -m 0750 /usr/local/etc/cortexdb
sudo install -d -m 0750 /usr/local/var/log/cortexdb
```

Create `/usr/local/etc/cortexdb/auth.tokens`:

```text
admin:replace-admin-token
data:replace-data-token
```

Protect it:

```bash
sudo chmod 0600 /usr/local/etc/cortexdb/auth.tokens
```

## Plist

The checked-in example lives at:

```text
docs/deployment/com.cortexdb.server.plist
```

Install it as a system daemon:

```bash
sudo cp docs/deployment/com.cortexdb.server.plist \
  /Library/LaunchDaemons/com.cortexdb.server.plist
sudo chown root:wheel /Library/LaunchDaemons/com.cortexdb.server.plist
sudo chmod 0644 /Library/LaunchDaemons/com.cortexdb.server.plist
```

The plist uses:

```text
ProgramArguments=/usr/local/bin/cortex-server /usr/local/var/cortexdb 127.0.0.1:8181
EnvironmentVariables.CORTEXDB_AUTH_TOKENS_FILE=/usr/local/etc/cortexdb/auth.tokens
EnvironmentVariables.CORTEXDB_ACTOR_QUEUE_CAPACITY=1024
EnvironmentVariables.CORTEXDB_RATE_LIMIT_PER_MINUTE=6000
EnvironmentVariables.CORTEXDB_AUDIT_LOG_FILE=/usr/local/var/log/cortexdb/audit.jsonl
StandardOutPath=/usr/local/var/log/cortexdb/server.log
StandardErrorPath=/usr/local/var/log/cortexdb/server.error.log
RunAtLoad=true
KeepAlive=true
```

## Logs

The launchd example writes process stdout/stderr to:

```text
/usr/local/var/log/cortexdb/server.log
/usr/local/var/log/cortexdb/server.error.log
```

Security/audit events are written separately through the configured audit sink:

```text
CORTEXDB_AUDIT_LOG_FILE=/usr/local/var/log/cortexdb/audit.jsonl
```

## Lifecycle

Load and start:

```bash
sudo launchctl bootstrap system /Library/LaunchDaemons/com.cortexdb.server.plist
sudo launchctl kickstart -k system/com.cortexdb.server
sudo launchctl print system/com.cortexdb.server
```

Stop and unload:

```bash
sudo launchctl bootout system/com.cortexdb.server
```

## Health And Validation

```bash
curl -H 'authorization: Bearer replace-admin-token' \
  http://127.0.0.1:8181/v1/health
curl -H 'authorization: Bearer replace-admin-token' \
  http://127.0.0.1:8181/v1/validate
```

Before upgrades, stop writes and run the rollback workflow in
[`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md). Backup and restore drills remain
operator CLI workflows, not launchd jobs.
