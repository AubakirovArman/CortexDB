FROM rust:1-bookworm AS build
WORKDIR /app
COPY . .
RUN cargo build --release -p cortex-server -p cortex-cli

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 cortexdb
COPY --from=build /app/target/release/cortex-server /usr/local/bin/cortex-server
COPY --from=build /app/target/release/cortexdb /usr/local/bin/cortexdb
USER cortexdb
WORKDIR /data
EXPOSE 8181
ENTRYPOINT ["cortex-server"]
CMD ["/data", "0.0.0.0:8181"]
