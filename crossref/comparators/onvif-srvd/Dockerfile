# onvif-srvd reference device container
# Source: https://github.com/KoynovStas/onvif_srvd
# Pinned commit: df09d0ac77b1f6540a32a5fab92e1226ad2a2f9d (2025-01-08)
#
# Digest-pinning deferred: base image digests below are pinned by tag.
# Operator should finalize by replacing tags with @sha256:... digests
# once a build environment with docker manifest inspect is available,
# mirroring the oracle's approach.

# ---------------------------------------------------------------------------
# Stage 1: build
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS build

# Install build toolchain and gSOAP system package (USE_SYSTEM_GSOAP=1).
# Using system gSOAP avoids the wget-based download in build_gsoap.cmake,
# which requires SourceForge/GitHub network access at build time.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        cmake \
        flex \
        bison \
        byacc \
        m4 \
        make \
        git \
        ca-certificates \
        gsoap \
        libgsoap-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

# Clone pinned commit of onvif_srvd
ARG ONVIF_SRVD_SHA=df09d0ac77b1f6540a32a5fab92e1226ad2a2f9d
RUN git clone https://github.com/KoynovStas/onvif_srvd.git /src/onvif_srvd \
    && git -C /src/onvif_srvd checkout ${ONVIF_SRVD_SHA}

WORKDIR /src/onvif_srvd

# Build using system gSOAP (variant 3 from README).
# -DUSE_SYSTEM_GSOAP=1 instructs cmake/FindgSOAP.cmake to find the installed
# gsoap/libgsoap-dev package instead of downloading gSOAP from the internet.
RUN cmake -B build . -DUSE_SYSTEM_GSOAP=1 -DCMAKE_BUILD_TYPE=Release \
    && cmake --build build --parallel "$(nproc)"

# ---------------------------------------------------------------------------
# Stage 2: runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install only runtime shared libs needed by onvif_srvd.
# gsoap (shared lib) is needed because we did NOT use -DUSE_GSOAP_STATIC_LIB.
# nc (netcat-openbsd) is present in netcat-traditional; use nc for healthcheck.
RUN apt-get update && apt-get install -y --no-install-recommends \
        gsoap \
        netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /src/onvif_srvd/build/onvif_srvd /usr/local/bin/onvif_srvd

# Service port — default onvif_srvd port is 1000; we expose it.
EXPOSE 1000

# Device info values matching the §7 fixture for srvd_exact GetDeviceInformation:
#   Manufacturer=Crossref  Model=Controlled-1  FirmwareVersion=1.0.0
#   SerialNumber=CR-0001   HardwareId=CR-HW-1
#
# --ifs eth0  — bind to the container's primary interface.
# --no_fork   — stay in foreground (required for Docker PID-1).
# --no_chdir  — do not chdir to /.
# --no_close  — keep stdio open (container logs).
#
# A minimal media profile is required; onvif_srvd refuses to start without
# at least one profile (see check_service_ctx). We supply a dummy RTSP URL;
# the orchestrator's XAddr masking means the host in this URL is irrelevant
# for projection comparisons.
#
# PTZ is enabled so that GetCapabilities returns a PTZ XAddr entry,
# increasing structural coverage for GetCapabilities projection.
ENTRYPOINT ["/usr/local/bin/onvif_srvd", \
    "--no_fork", \
    "--no_chdir", \
    "--no_close", \
    "--port",         "1000", \
    "--manufacturer", "Crossref", \
    "--model",        "Controlled-1", \
    "--firmware_ver", "1.0.0", \
    "--serial_num",   "CR-0001", \
    "--hardware_id",  "CR-HW-1", \
    "--scope",        "onvif://www.onvif.org/Profile/S", \
    "--scope",        "onvif://www.onvif.org/name/Crossref", \
    "--ifs",          "eth0", \
    "--name",         "Profile1", \
    "--width",        "1920", \
    "--height",       "1080", \
    "--url",          "rtsp://%s:554/stream", \
    "--type",         "H264", \
    "--ptz" \
]

# Healthcheck: TCP probe on port 1000 using nc (netcat-openbsd).
# nc -z performs a connect-only probe; exits 0 on success.
HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=3 \
    CMD nc -z 127.0.0.1 1000
