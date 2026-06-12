# CortexDB Docker Hardening

Status: local single-node container example.

The Docker image is an operator convenience for the local blocking CortexDB core
behind the async HTTP server. It is not a managed cloud, Kubernetes, or
distributed deployment recipe.

## Runtime Boundary

The checked-in `Dockerfile` uses:

```text
runtime user: 10001:10001
database path: /data
server bind: 0.0.0.0:8181 inside the container
healthcheck: GET /v1/health
```

The runtime image pre-creates `/data` with owner `10001:10001` and mode `0750`.
The process runs as `USER 10001:10001`, not as root.

## Compose Hardening

The checked-in `docker-compose.yml` is the production-like local example. It
sets:

```text
user: "10001:10001"
read_only: true
tmpfs: /tmp:rw,noexec,nosuid,size=64m
security_opt: no-new-privileges:true
cap_drop: ALL
volume: cortexdb-data:/data:rw
healthcheck: curl -sf http://localhost:8181/v1/health
```

With `read_only: true`, `/data` is the intended writable database volume.

## Volume Permission Check

Named Docker volumes should inherit the `/data` ownership from the image on
first use. For bind mounts, create the directory with the container UID/GID:

```bash
mkdir -p ./data
sudo chown 10001:10001 ./data
sudo chmod 0750 ./data
```

Before using a bind mount in production-like tests, verify it is writable by
the container user:

```bash
docker run --rm --user 10001:10001 -v "$PWD/data:/data:rw" \
  --entrypoint sh cortexdb:local -c 'test -w /data'
```

## Local Evidence Gate

The Docker hardening gate is intentionally daemon-free; it validates the
checked-in contract without requiring Docker on CI hosts:

```bash
make docker-hardening-check
```

The report is written to:

```text
target/docker-hardening/report.json
```

## Production Compose Example

`docker-compose.production.yml` is the checked-in production-like local
topology example. It keeps the database server unexposed on the host and puts a
reverse proxy in front:

```text
host:8181 -> reverse-proxy:8080 -> cortexdb:8181
```

The production example includes:

```text
reverse-proxy: nginx:1.27-alpine
auth: CORTEXDB_AUTH_TOKENS_FILE=/run/secrets/cortexdb-auth.tokens
data volume: cortexdb-data:/data:rw
backup sidecar: backup-sidecar profile maintenance
backup volume: cortexdb-backups:/backups:rw
```

Before starting it, create a local token file. Do not commit real token values:

```bash
mkdir -p ./secrets
cp docs/deployment/auth.tokens.example ./secrets/auth.tokens
chmod 0600 ./secrets/auth.tokens
$EDITOR ./secrets/auth.tokens
```

Start the server and reverse proxy:

```bash
docker compose -f docker-compose.production.yml up -d cortexdb reverse-proxy
```

Call the API through the reverse proxy with a token from `./secrets/auth.tokens`:

```bash
curl -H "Authorization: Bearer <token>" http://127.0.0.1:8181/v1/health
```

The backup sidecar is a maintenance example. It uses the same database volume
and a separate backup volume. Stop the main server first so the backup command
can acquire the database lock:

```bash
docker compose -f docker-compose.production.yml stop cortexdb
docker compose -f docker-compose.production.yml run --rm backup-sidecar
docker compose -f docker-compose.production.yml start cortexdb
```

The production compose gate is daemon-free and validates the checked-in
topology contract:

```bash
make docker-production-compose-check
```

The report is written to:

```text
target/docker-production-compose/report.json
```
