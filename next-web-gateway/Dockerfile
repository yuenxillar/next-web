# Build the gateway from the workspace.
#
# The build context is the repository root (the workspace `Cargo.toml` and
# `Cargo.lock` must be present):
#   docker build -f next-web-gateway/Dockerfile -t next-web-gateway .
FROM rust:1.98-slim AS builder

# pingora pulls flate2 with the zlib-ng backend, which needs cmake and a C toolchain.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        build-essential cmake nasm pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release --locked -p next-web-gateway --bin next-web-gateway

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 gateway

COPY --from=builder /build/target/release/next-web-gateway /usr/local/bin/next-web-gateway
COPY --from=builder /build/next-web-gateway/application.yaml /etc/next-web-gateway/application.yaml

USER gateway
EXPOSE 8080 9001

ENTRYPOINT ["/usr/local/bin/next-web-gateway"]
CMD ["--config", "/etc/next-web-gateway/application.yaml"]
