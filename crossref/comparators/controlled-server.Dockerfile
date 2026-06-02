# controlled-server image — multi-stage Rust build from the repo root context.
#
# BUILD CONTEXT: repo root (../  relative to crossref/docker-compose.yml).
#
# STAGING REQUIREMENT (Task 6 must do this before docker compose ... up --build):
#   rsync -a --exclude=target/ ../../soap-server/ crossref/.build/soap-server/
# The build COPYs crossref/.build/soap-server into /build/soap-server so that
# the crossref/Cargo.toml path dep  "soap-server = { path = '../../soap-server' }"
# resolves from /build/onvif-server/crossref → /build/soap-server (correct).
#
# Digest-pinning: base image tags are used here; operator should finalize by
# replacing tags with @sha256:<digest> (use `docker manifest inspect <image>`).
# The rust:1.85 tag matches the rust-toolchain.toml channel = "1.85.1" in this
# repo; using a newer base would cause rustup to download a different toolchain.

# ---------------------------------------------------------------------------
# Stage 1: build
# ---------------------------------------------------------------------------
FROM rust:1.85 AS build

WORKDIR /build

# Staged soap-server (populated by Task-6 orchestrator before compose up --build).
# Path math: Cargo.toml at /build/onvif-server/crossref/Cargo.toml has
#   soap-server = { path = "../../soap-server" }
# ../../soap-server from /build/onvif-server/crossref → /build/soap-server. Correct.
COPY crossref/.build/soap-server /build/soap-server

# Copy the onvif-server workspace (repo root).
COPY . /build/onvif-server

WORKDIR /build/onvif-server

RUN cargo build -p onvif-crossref --bin controlled_onvif_server --release

# ---------------------------------------------------------------------------
# Stage 2: runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

COPY --from=build /build/onvif-server/target/release/controlled_onvif_server /usr/local/bin/controlled_onvif_server

EXPOSE 8080

# debian:bookworm-slim includes bash; TCP probe requires no extra packages.
HEALTHCHECK --interval=2s --timeout=2s --retries=30 \
    CMD bash -c "echo > /dev/tcp/localhost/8080"

ENTRYPOINT ["controlled_onvif_server"]
