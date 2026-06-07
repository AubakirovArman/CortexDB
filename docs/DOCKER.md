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
