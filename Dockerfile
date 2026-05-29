FROM rust:1-bookworm AS build
WORKDIR /app

# Cache dependencies for faster rebuilds
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo build --release -p cortex-server -p cortex-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 cortexdb

COPY --from=build /app/target/release/cortex-server /usr/local/bin/cortex-server
COPY --from=build /app/target/release/cortexdb /usr/local/bin/cortexdb

USER cortexdb
WORKDIR /data
EXPOSE 8181

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -sf http://localhost:8181/v1/health || exit 1

ENTRYPOINT ["cortex-server"]
CMD ["/data", "0.0.0.0:8181"]
