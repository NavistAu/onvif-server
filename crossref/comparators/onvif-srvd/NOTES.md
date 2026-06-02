# onvif-srvd reference device — build & comparison notes

## Chosen project

| Field | Value |
|---|---|
| Project | KoynovStas/onvif_srvd — "ONVIF Device(IP camera) Service server (Linux daemon)" |
| URL | https://github.com/KoynovStas/onvif_srvd |
| Pinned SHA | `df09d0ac77b1f6540a32a5fab92e1226ad2a2f9d` (master, 2025-01-08) |
| License | GPLv2 |

### Why this project

It is the canonical open-source gSOAP-based ONVIF Profile-S server widely referenced for
testing. Its source directly sets `tds__GetDeviceInformationResponse` fields from CLI
flags, which means we can pin all five device-info values from the command line without
patching source. No alternatives were evaluated; the project cleanly satisfies all
requirements.

---

## Build dependencies

System packages (Debian bookworm):

- `build-essential` `cmake` `flex` `bison` `byacc` `m4` `make` `git` `ca-certificates`
- `gsoap` `libgsoap-dev` — system gSOAP avoids the cmake wget-based download (cmake
  variant 3 from README: `-DUSE_SYSTEM_GSOAP=1`).

Runtime packages:

- `gsoap` (shared library)
- `netcat-openbsd` (healthcheck TCP probe)

### Why system gSOAP (not self-built)

The default cmake build (`build_gsoap.cmake`) downloads `gsoap_2.8.92.zip` from
SourceForge/GitHub at build time. In a locked-down CI environment that is fragile.
Using `-DUSE_SYSTEM_GSOAP=1` with `apt install gsoap libgsoap-dev` (Debian bookworm ships
gSOAP 2.8.117) is more reproducible and avoids the network dependency.

---

## Device-info configuration

All five fields are set via CLI flags in the ENTRYPOINT; no source modification required:

```
--manufacturer  Crossref
--model         Controlled-1
--firmware_ver  1.0.0
--serial_num    CR-0001
--hardware_id   CR-HW-1
```

These map directly to `ServiceContext` fields set in `processing_cmd()` and emitted
verbatim from `DeviceBindingService::GetDeviceInformation`.

---

## Digest pinning

Base images are pinned by tag (`debian:bookworm-slim`) only. Digest-pinning (the `@sha256:`
form used in the oracle Dockerfile) is deferred: it requires `docker manifest inspect` or
a registry query at pinning time, which is unavailable in this static-inspection task.
Operator should finalize by running:

```sh
docker pull debian:bookworm-slim
docker inspect --format='{{index .RepoDigests 0}}' debian:bookworm-slim
```

and replacing the FROM lines with `debian:bookworm-slim@sha256:<digest>`.

---

## XAddr path mismatch (CRITICAL — read before Task 6)

`getXAddr()` in `ServiceContext.cpp` returns `http://<ip>:<port>` — just the authority, no
path. Every service XAddr in GetCapabilities and GetServices is therefore **bare authority**,
e.g. `http://172.17.0.2:1000` with no path component.

Our controlled server emits paths: `/onvif/device_service`, `/onvif/media_service`,
`/onvif/ptz_service`, etc.

After the orchestrator masks host authority (the `host_authority` mask strips the host:port
prefix), the remaining comparison is:

| Our fixture (path) | srvd (path) |
|---|---|
| `/onvif/device_service` | `` (empty — bare authority has no path) |
| `/onvif/media_service` | `` (empty) |
| `/onvif/ptz_service` | `` (empty) |

This is a **structural mismatch in the XAddr paths** for both GetCapabilities and
GetServices. The projection comparison will fail unless the framework treats an empty path
as a wildcard — which is unlikely. This should be expected to yield `ReferenceDisagreement`
verdicts for those two ops in Task 6.

**Recommendation for Task 6 triage:** If the projection comparator does not project away
the XAddr path component, downgrade `device_get_capabilities` and `device_get_services` to
`reference_mode = "none"` (oracle + invariants only).

---

## GetServices — additional structural differences

srvd emits 3 services: Device (v16.12), Media (v2.6), PTZ (v2.4) — only when PTZ is
enabled, and only when `ptz_node.enable` is checked (note: the source has a logic inversion
bug: it returns early if `ptz_node.enable` is true, so PTZ service is only appended when
PTZ is **disabled**; confirmed by reading `ServiceDevice.cpp` lines 68-73):

```cpp
if(ctx->get_ptz_node()->enable)
    return SOAP_OK;          // <-- exits before appending PTZ service

//PTZ Service
auto ptz_svc = ...
tds__GetServicesResponse.Service.emplace_back(ptz_svc);
```

With `--ptz` passed (as in our Dockerfile), srvd will emit **only Device + Media** (2
services, no PTZ). Our fixture has 5 services (Device, Media, PTZ, Imaging, Events).

Our Dockerfile passes `--ptz` to get the PTZ XAddr in GetCapabilities — but that causes
GetServices to return only 2 services instead of 3. To get 3 services from GetServices,
omit `--ptz`; but then GetCapabilities has no PTZ entry.

This is a confirmed bug in the upstream source. We cannot simultaneously satisfy both.
The Dockerfile keeps `--ptz` for GetCapabilities coverage; GetServices will have 2 entries
vs our 5.

**Recommendation for Task 6:** Both `device_get_capabilities` and `device_get_services`
will produce `ReferenceDisagreement` (path mismatch + service count mismatch). Downgrade
both to `reference_mode = "none"` in Task 6.

---

## GetProfiles — structural differences

srvd builds Profile objects from `StreamProfile::get_profile()`. The structure differs
from our fixture in several ways:

- **Token naming**: srvd uses the profile `name` as the token. With `--name Profile1`, the
  token will be `Profile1` vs our `profile_0`.
- **PTZConfiguration**: srvd includes a PTZ config in the profile when `--ptz` is set.
  Field presence broadly matches ours, but attribute values will differ (NodeToken, space
  URIs).
- **VideoSourceConfiguration/VideoEncoderConfiguration**: srvd populates these from the
  profile parameters; names and tokens differ from our fixture.
- **SessionTimeout**: srvd emits `PT60S`; our fixture has `PT10S`.
- **Multicast block**: srvd omits Multicast element; our fixture includes it.

The projection comparator masks fields it can't compare structurally. Whether these
differences produce `ReferenceDisagreement` depends on what fields the projection actually
compares. Moderate mismatch risk.

**Recommendation for Task 6:** If GetProfiles projection fails, downgrade
`media_get_profiles` to `reference_mode = "none"`.

---

## GetDeviceInformation — srvd_exact assessment

This is the only `srvd_exact` op. All five fields are set directly from CLI flags with no
intermediate transformation. The gSOAP serialization emits them as plain XML text children
of `GetDeviceInformationResponse` in the order: Manufacturer, Model, FirmwareVersion,
SerialNumber, HardwareId — which matches the ONVIF devicemgmt.wsdl schema element order
and our fixture exactly.

**Assessment: HIGH confidence this op will pass srvd_exact.**

The only risk is XML namespace prefix differences (our fixture uses `n0:`, gSOAP uses its
own prefix assignments). The `srvd_exact` mode applies masks before comparison; confirm
the framework's masked structural equality handles namespace prefixes.

---

## §6 op assessment summary

| Op | reference_mode | Confidence | Notes |
|---|---|---|---|
| GetDeviceInformation | `srvd_exact` | HIGH PASS | All 5 fields set via CLI; order matches |
| GetCapabilities | `srvd_projection` | HIGH FAIL | XAddr paths empty vs `/onvif/...`; service count |
| GetServices | `srvd_projection` | HIGH FAIL | XAddr paths empty; only 2 services vs 5; PTZ bug |
| GetProfiles | `srvd_projection` | MODERATE FAIL | Token/name mismatch, missing Multicast, session timeout differs |

**Task 6 expected actions:**
- Keep `device_get_device_information_authed` as `srvd_exact` (expected to pass).
- Downgrade `device_get_capabilities`, `device_get_services`, `media_get_profiles` to
  `reference_mode = "none"` if projection comparison fails (expected).
