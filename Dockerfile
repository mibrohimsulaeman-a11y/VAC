# =============================================================================
# VAC local development image
# =============================================================================
#
# This Dockerfile is a local build recipe for the VAC v1.9 source tree. It does
# not claim a published registry image. Build locally with:
#
#   docker build -t vac-local:dev .
#
# Cargo build/test status must be verified locally with --manifest-path vac-rs/Cargo.toml before release claims.
# =============================================================================

FROM rust:1.94.1-slim-bookworm AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/vac
COPY . .
RUN cargo build --release --manifest-path vac-rs/Cargo.toml -p vac-cli --target-dir /usr/src/vac/target
RUN strip /usr/src/vac/target/release/vac || true

FROM python:3.13-slim-bookworm
LABEL org.opencontainers.image.source="local-vac-v1-9-source" \
    org.opencontainers.image.description="VAC local development image" \
    maintainer="Vastar AI"

RUN apt-get update -y && apt-get install -y \
    curl unzip git ca-certificates gnupg netcat-traditional wget dnsutils sudo gosu \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/vac/target/release/vac /usr/local/bin/vac
RUN chmod +x /usr/local/bin/vac

RUN groupadd -g 1000 agent && useradd -u 1000 -g 1000 -s /bin/bash -m agent \
    && mkdir -p /workspace /home/agent/.vac/data /home/agent/.agent-board /home/agent/.cache/vac \
    && chown -R agent:agent /workspace /home/agent/.vac /home/agent/.agent-board /home/agent/.cache/vac

COPY --chown=agent:agent scripts/entrypoint.sh /home/agent/.local/bin/entrypoint.sh
RUN chmod +x /home/agent/.local/bin/entrypoint.sh

USER agent
WORKDIR /workspace
ENTRYPOINT ["/home/agent/.local/bin/entrypoint.sh", "/usr/local/bin/vac"]
