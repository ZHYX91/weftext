# Build context: repository root. Docker then selects the adjacent
# deploy/Dockerfile.dockerignore allowlist for this non-default Dockerfile:
# docker build --file crates/weftext-server/deploy/Dockerfile .
FROM rust:1.98.0-bookworm@sha256:e536cf316987faedfe8ae120f83b70c7df0068fdb4fc9efcce55c71a625001d5 AS build
WORKDIR /source
COPY . .
RUN cargo build --locked --release -p weftext-server

FROM busybox:1.37.0-musl@sha256:222ad6d973c0d198014546a65cd02c5fdedcc172123c5b4c2bf0af636550bd94 AS healthcheck

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241
COPY --from=build /source/target/release/weftext-server /usr/local/bin/weftext-server
COPY --from=healthcheck /bin/busybox /usr/local/bin/busybox
USER 10001:10001
VOLUME ["/workspace", "/var/lib/weftext"]
STOPSIGNAL SIGTERM
ENTRYPOINT ["/usr/local/bin/weftext-server"]
CMD ["/workspace", "--control-plane", "/var/lib/weftext", "--bind", "127.0.0.1:8787"]
