# CortexDB Systemd Service

Status: Core Alpha example for a single local node.

This service wraps the blocking single-node database core with the async HTTP
server. It is not a distributed deployment recipe.

## Layout

Suggested paths:

```text
/usr/local/bin/cortex-server
/var/lib/cortexdb
/etc/cortexdb/cortexdb.env
/etc/cortexdb/auth.tokens
```

Create the runtime user:

```bash
useradd --system --home /var/lib/cortexdb --shell /usr/sbin/nologin cortexdb
install -d -o cortexdb -g cortexdb -m 0750 /var/lib/cortexdb
install -d -o root -g cortexdb -m 0750 /etc/cortexdb
```

## Environment File

`/etc/cortexdb/cortexdb.env`:

```bash
CORTEXDB_AUTH_TOKENS_FILE=/etc/cortexdb/auth.tokens
CORTEXDB_ACTOR_QUEUE_CAPACITY=1024
CORTEXDB_REQUEST_RATE_LIMIT_PER_SECOND=100
CORTEXDB_AUDIT_LOG_PATH=/var/lib/cortexdb/audit.jsonl
```

`/etc/cortexdb/auth.tokens`:

```text
admin:replace-admin-token
data:replace-data-token
```

Protect both files:

```bash
chown root:cortexdb /etc/cortexdb/cortexdb.env /etc/cortexdb/auth.tokens
chmod 0640 /etc/cortexdb/cortexdb.env /etc/cortexdb/auth.tokens
```

## Unit File

`/etc/systemd/system/cortexdb.service`:

```ini
[Unit]
Description=CortexDB single-node server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=cortexdb
Group=cortexdb
EnvironmentFile=/etc/cortexdb/cortexdb.env
ExecStart=/usr/local/bin/cortex-server /var/lib/cortexdb 127.0.0.1:8181
Restart=on-failure
RestartSec=2s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/lib/cortexdb

[Install]
WantedBy=multi-user.target
```

The checked-in example is:

```text
docs/deployment/cortexdb.service
```

`make service-manager-smoke-check` validates this unit and the macOS launchd
plist before deployment evidence is accepted.

Enable and start:

```bash
systemctl daemon-reload
systemctl enable --now cortexdb
systemctl status cortexdb
```

## Health And Validation

```bash
curl -H 'authorization: Bearer replace-admin-token' \
  http://127.0.0.1:8181/v1/health
curl -H 'authorization: Bearer replace-admin-token' \
  http://127.0.0.1:8181/v1/validate
```

Before upgrades, stop writes and run the rollback workflow in
[`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md).
