# Project Research Summary

**Project:** onvif-server
**Domain:** Rust ONVIF device server crate (SOAP/ONVIF layer for virtual PTZ cameras)
**Researched:** 2026-04-05
**Confidence:** HIGH

## Executive Summary

`onvif-server` is a Rust library crate that exposes a trait-based API for building virtual ONVIF camera devices. The primary downstream consumer is Frigate's PTZ autotracker, which requires a specific and non-obvious subset of ONVIF operations — particularly `TranslationSpaceFov` space advertisement, `MoveStatus` capability, and strict profile structure — to function at all. The core transport layer is already implemented in the sibling `soap-server` crate (SOAP envelope parsing, WS-Security, dispatch, WSDL serving). `onvif-server`'s job is exclusively to wire ONVIF service traits to that transport, provide the correct ONVIF XML response structures, and bundle the necessary WSDLs.

The recommended approach is a per-service `SoapHandler` adapter pattern: each ONVIF service (Device, Media, PTZ, Imaging, Events) becomes a separate `SoapService` backed by a trait that consumers implement. Service traits provide `not_implemented()` defaults for optional operations so consumers only implement what they support. Type definitions should initially come from `lumeohq/onvif-rs` schema git crates (Option A), falling back to generating types from `xsd-parser 1.5` in `build.rs` if onvif-rs types prove broken or unmaintained (Option B). The build order is driven by architecture: errors and types first, Device service next to validate the wiring pattern, then Media and PTZ for Frigate compatibility, Imaging and Events last.

The primary risks are Frigate-specific compatibility traps that fail silently: a PTZ profile lacking `DefaultContinuousPanTiltVelocitySpace`, a missing `TranslationSpaceFov` URI in `GetConfigurationOptions`, or a `GetServiceCapabilities` response without `MoveStatus="true"` will each cause Frigate to disable autotracking with no meaningful error. Secondary risks are in the WS-Security layer: `GetSystemDateAndTime` must be auth-exempt, and nonce/timestamp formatting must follow specific constraints or strict cameras reject authentication entirely. These are not implementation complexity problems — they are precision problems with known fixes that must be encoded as integration tests from the start.

## Key Findings

### Recommended Stack

The entire SOAP transport exists in `soap-server` (sibling crate). `onvif-server` adds only the ONVIF-specific layer on top. The key dependency decision is type generation strategy: use `lumeohq/onvif-rs` schema crates via git dependency first (pre-generated, validated types), with `xsd-parser 1.5` + `build.rs` as fallback. The yaserde version mismatch between onvif-rs (0.7) and the current crate (0.12) must be investigated before committing to Option A — Cargo may or may not resolve this cleanly.

**Core technologies:**
- `soap-server` (path dep): SOAP transport, WS-Security, dispatch — must not be re-implemented
- `tokio 1` + `axum 0.8`: async runtime and HTTP routing — must match soap-server versions exactly
- `async-trait 0.1`: required for `Arc<dyn XService>` dynamic dispatch (native AFIT does not support dyn in 2025)
- `yaserde 0.12` (or match onvif-rs pin): XML de/serialize for ONVIF types
- `thiserror 2`: error derivation, consistent with soap-server dependency chain
- `uuid 1` + `chrono 0.4`: device serial numbers and UTC timestamp responses
- `socket2 0.5`: UDP multicast setup for WS-Discovery (optional `discovery` feature only)

### Expected Features

The feature set is precisely defined by Frigate's autotracker call sequence and ONVIF Device Manager smoke tests. The 10 critical Frigate-compatibility pitfalls reduce to a well-understood checklist; there is no ambiguity about what v1 needs to do.

**Must have (table stakes — clients break without these):**
- `GetSystemDateAndTime` (auth-exempt) — prerequisite for all WS-Security digest auth
- `GetCapabilities` + `GetServices` (both required) — discovery by legacy and modern clients
- `GetDeviceInformation`, `GetScopes` — basic identity
- `GetProfiles`, `GetStreamUri`, `GetVideoSources`, `GetVideoSourceConfigurations`, `GetVideoEncoderConfigurations` — Profile S media
- `PTZ.GetNodes` with `TranslationSpaceFov`, `GetConfigurations`, `GetConfiguration`, `GetConfigurationOptions` with FOV space, `GetServiceCapabilities` with `MoveStatus="true"`
- `PTZ.RelativeMove`, `ContinuousMove`, `Stop`, `GetStatus`, `GetPresets`, `GotoPreset` — full PTZ control surface
- `Imaging.GetImagingSettings` — Frigate reads at startup
- WS-Security UsernameToken digest auth (via soap-server)
- `OnvifServer::builder()` type-safe builder API

**Should have (differentiators — add post-Frigate validation):**
- `GetSnapshotUri` — Home Assistant integration
- Events service (`CreatePullPointSubscription`, `PullMessages`) — HA motion detection
- `GetHostname`, `GetNetworkInterfaces` — ODM compatibility
- `AbsoluteMove` — Frigate zoom control
- Frigate compatibility test suite in `tests/frigate_compat.rs`

**Defer (v2+):**
- WS-Discovery `discovery` feature flag — no NVR auto-discovery use case yet
- Media2 / Profile T — no H.265 client requirement yet
- Profile G (recording services) — entirely different domain

### Architecture Approach

`onvif-server` is purely a translation layer: it bridges soap-server's raw-bytes `SoapHandler` interface to typed Rust service traits that consumers implement. One `ServerBuilder` + `SoapService` per ONVIF WSDL, merged via `Router::merge()`. Consumers implement only the subset of trait methods they support; all others return spec-correct `ActionNotSupported` faults via default implementations. WSDLs and XSDs are embedded at compile time via `include_bytes!` and served by a `WsdlLoader` implementation.

**Major components:**
1. `OnvifServer` builder (`server.rs`) — wires per-service `SoapService` instances, configures auth-bypass, merges axum routers
2. Service traits + `ServiceRouter` (per-service in `traits/`) — defines consumer API, bridges bytes-in/out to typed async trait calls
3. `EmbeddedWsdlLoader` (`wsdl_loader.rs`) — serves bundled WSDL/XSD bytes to soap-server at startup
4. `OnvifError` + `not_implemented()` (`error.rs`) — maps trait errors to spec-correct `SoapFault` responses
5. `discovery` module (feature-gated) — UDP multicast WS-Discovery as a separate tokio task

### Critical Pitfalls

1. **`GetSystemDateAndTime` must be auth-exempt** — if WS-Security is enforced here, clients cannot compute any subsequent digest and all authentication fails. Register this operation in soap-server's `auth_bypass` list unconditionally. Verify with a test that sends no `Security` header and expects HTTP 200.

2. **Frigate silently drops PTZ if `GetProfiles` profile lacks `DefaultContinuousPanTiltVelocitySpace`** — every profile must have `PTZConfiguration.DefaultContinuousPanTiltVelocitySpace` set to the `TranslationSpaceFov` URI, or Frigate logs "no appropriate ONVIF profiles found" and disables PTZ entirely.

3. **`TranslationSpaceFov` URI must be byte-for-byte exact in both `GetNodes` and `GetConfigurationOptions`** — define `pub const TRANSLATION_SPACE_FOV: &str = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov"` and use it everywhere. A typo or trailing slash breaks Frigate autotracking silently.

4. **`GetServiceCapabilities` must return `MoveStatus="true"` or Frigate never polls `GetStatus`** — without this, Frigate's autotracker never enters the status-polling loop and overshoots every target. Do not let this operation hit the default `not_implemented()` handler.

5. **Token inconsistency across services causes `NoProfile` faults** — profile token, PTZ configuration token, and PTZ node token are referenced across `GetProfiles`, `GetConfigurations`, `GetNodes`, `RelativeMove`, and `GetStatus`. Define all as crate-level constants from day one; hardcoded inline strings will diverge.

6. **WS-Security nonce/timestamp format rejects strict cameras** — nonce must be ~28 base64 chars (20 raw random bytes, not UUID string); `Created` must be millisecond precision ending in `Z` (`2024-02-04T07:41:57.802Z`), not nanoseconds or timezone offsets. Non-compliance causes HikVision/Dahua authentication to fail.

7. **SOAP faults must declare `xmlns:ter`** — every fault envelope must include `xmlns:ter="http://www.onvif.org/ver10/error"` or python-zeep (used by Frigate) throws `XMLParseError`. This affects all default `not_implemented()` faults.

## Implications for Roadmap

Based on combined research, the architecture's component dependency graph and the Frigate-first validation strategy suggest 5 phases:

### Phase 1: Foundation — Error types, WSDL loader, project skeleton
**Rationale:** `OnvifError`, `not_implemented()`, `DeviceInfo`, and `EmbeddedWsdlLoader` have no dependencies on other onvif-server components. Everything downstream depends on them. Setting these up with correct SOAP fault namespacing (ter: pitfall) prevents regressions from the start.
**Delivers:** Compilable crate scaffold; spec-compliant SOAP fault infrastructure; embedded WSDL/XSD bundle; token constants
**Addresses:** Builder API scaffolding; anti-pattern prevention for inline token strings
**Avoids:** SOAP fault `ter:` namespace pitfall (Pitfall 3); token inconsistency pitfall (Pitfall 9)

### Phase 2: Device Management Service + WS-Security wiring
**Rationale:** Every ONVIF client's first call is `GetSystemDateAndTime` followed by `GetCapabilities` or `GetServices`. This phase validates end-to-end: embedded WSDL loads, `ServerBuilder` dispatch table builds, auth-bypass is registered, and the axum router serves requests. It is the prerequisite for all subsequent service phases.
**Delivers:** Working ONVIF device endpoint; auth-exempt `GetSystemDateAndTime`; `GetCapabilities` + `GetServices` (both); `GetDeviceInformation`; `GetScopes`
**Uses:** `soap-server` `ServerBuilder`, `auth_bypass`, `from_wsdl_bytes_with_loader`
**Implements:** `OnvifServer` builder, `DeviceServiceHandler` (ServiceRouter pattern prototype)
**Avoids:** GetSystemDateAndTime auth pitfall (Pitfall 1); nonce/timestamp format pitfall (Pitfall 2); GetCapabilities + GetServices both required (Pitfall 10)

### Phase 3: Media Service — Profile S streaming metadata
**Rationale:** Profile tokens established here thread through every subsequent PTZ operation. Media must be correct before PTZ can be tested. `GetProfiles` is where the Frigate profile-structure pitfall lives; fixing it here before wiring PTZ prevents a confusing failure mode where PTZ is implemented but Frigate still won't use it.
**Delivers:** `GetProfiles` (with correct PTZConfiguration structure), `GetStreamUri`, `GetVideoSources`, `GetVideoSourceConfigurations`, `GetVideoEncoderConfigurations`; validates multi-service router merging
**Uses:** Profile token constant from Phase 1; `Router::merge()` pattern
**Avoids:** Frigate profile structure pitfall (Pitfall 4); token consistency pitfall (Pitfall 9)

### Phase 4: PTZ Service — Frigate autotracker compatibility
**Rationale:** This is the core deliverable. All Frigate-specific pitfalls are in this phase. The build order within PTZ mirrors the client's startup sequence: discovery operations first (`GetNodes`, `GetConfigurations`, `GetConfigurationOptions`, `GetServiceCapabilities`), then movement operations (`RelativeMove`, `ContinuousMove`, `Stop`, `GetStatus`, `GetPresets`, `GotoPreset`). Frigate compatibility test suite (`tests/frigate_compat.rs`) must be created alongside this phase.
**Delivers:** Full PTZ service with `TranslationSpaceFov`, `MoveStatus`, and correct coordinate spaces; Frigate end-to-end validation
**Implements:** `PTZService` trait + `PTZServiceHandler`
**Avoids:** TranslationSpaceFov URI pitfall (Pitfall 5); MoveStatus pitfall (Pitfall 6); token inconsistency across PTZ ops (Pitfall 9)

### Phase 5: Imaging Service + polish
**Rationale:** `GetImagingSettings` is called by Frigate at startup but its absence doesn't break PTZ. Adding it after Frigate PTZ compatibility is confirmed avoids scope creep in the critical path. Includes the `virtual_ptz` example, ONVIF Device Manager smoke test, and any quick-win additional operations (`GetSnapshotUri`, `GetHostname`).
**Delivers:** `Imaging.GetImagingSettings`; `virtual_ptz` example; ODM smoke test; optional supplementary operations
**Addresses:** Table-stakes features not in Frigate's critical path

### Phase Ordering Rationale

- Phase 1 before everything: type aliases, error types, and token constants are prerequisites for all other modules, and establishing the `ter:` namespace in faults early prevents it from being retrofitted across all services later.
- Phase 2 before Phase 3: WSDL loading and auth wiring must be proven to work with a single service before adding the complexity of multi-service merging.
- Phase 3 before Phase 4: Profile tokens must be established before PTZ operations can reference them; the Frigate profile-structure pitfall must be resolved before PTZ testing begins.
- Phase 4 before Phase 5: Frigate PTZ compatibility is the primary success criterion for v1. Only once that is validated does adding the Imaging service and example matter.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1 (type strategy):** The yaserde 0.12 vs. onvif-rs pinned yaserde 0.7 version conflict needs investigation before committing to Option A. This may require a small spike: clone onvif-rs, add it as a git dep, attempt to compile a minimal usage, and verify the yaserde version resolves cleanly. If it does not, pivot to Option B (xsd-parser build.rs) early to avoid discovering this mid-Phase 2.
- **Phase 4 (PTZ):** The exact XML structure Frigate validates in `GetProfiles`, `GetNodes`, `GetConfigurationOptions`, and `GetServiceCapabilities` should be verified against the live Frigate source (`frigate/ptz/onvif.py`) before coding those response types. The hawkeye217 gist is a useful secondary reference but reading the actual source is authoritative.

Phases with standard patterns (skip research-phase):
- **Phase 2 (Device Management):** soap-server API is fully read; ONVIF spec auth rules are documented; no unknowns.
- **Phase 3 (Media):** Profile S response structures are well-specified; the pitfall is about Frigate's profile-selection logic which is already researched.
- **Phase 5 (Imaging/polish):** `GetImagingSettings` is a minimal, low-risk operation; virtual_ptz example follows the existing API.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Core dependencies verified directly against soap-server source; only unknowns are yaserde version compat with onvif-rs (MEDIUM) and tokio LTS definition (MEDIUM via search only) |
| Features | HIGH | Derived from ONVIF specs, Frigate source code, and python-onvif-zeep call sequence — primary sources throughout |
| Architecture | HIGH | Based on direct reading of soap-server source (`server.rs`, `handler.rs`, `dispatch.rs`) and DESIGN.md — highest confidence research area |
| Pitfalls | HIGH | Every critical pitfall is sourced from real issues (onvif-rs #114, python-zeep #1205, CVE-2022-30563) or spec text — not theoretical |

**Overall confidence:** HIGH

### Gaps to Address

- **yaserde version compatibility (Option A viability):** onvif-rs pins yaserde 0.7; onvif-server targets 0.12. Cargo may or may not unify these. This needs a 30-minute spike (add the git dep, attempt compile) before Phase 1 design is finalized. If it fails, Option B becomes the plan and Phase 1 scope expands to include `build.rs` type generation setup.
- **onvif-rs schema crate server-side usability:** The onvif-rs types were designed for client deserialization. Some type fields required for server-side serialization may be optional/missing. This surfaces during Phase 2-3 implementation. Mitigation: log raw XML of every response during development and validate with python-zeep.
- **soap-server `WsdlLoader` trait exact interface:** ARCHITECTURE.md references this trait but the exact signature (method names, error type) should be confirmed from soap-server source before writing `EmbeddedWsdlLoader` in Phase 1.
- **ONVIF Profile S deprecation timeline:** Profile S conformance ends March 2027 per ONVIF's announcement. v1 targets Profile S. If a consumer requires Profile T (Media2, H.265) before that date, v2 scope needs to expand. Not an immediate concern but worth flagging for roadmap long-term planning.

## Sources

### Primary (HIGH confidence)
- `/Users/jhogendorn/ws/soap-server/src/` — Direct reading of server.rs, handler.rs, dispatch.rs, lib.rs
- `docs/DESIGN.md` in soap-server — authoritative design intent
- [Frigate ptz/onvif.py](https://github.com/blakeblackshear/frigate/blob/dev/frigate/ptz/onvif.py) — ground truth for Frigate ONVIF call sequence and validation
- [ONVIF Core Specification v25.12](https://www.onvif.org/specs/core/ONVIF-Core-Specification.pdf) — auth rules, fault format, GetScopes
- [ONVIF PTZ Service Specification v25.12](https://www.onvif.org/specs/srv/ptz/ONVIF-PTZ-Service-Spec.pdf) — coordinate spaces, GetNodes, GetStatus
- [ONVIF Media Service Specification v24.12](https://www.onvif.org/specs/srv/media/ONVIF-Media-Service-Spec.pdf) — GetProfiles, GetStreamUri

### Secondary (MEDIUM confidence)
- [lumeohq/onvif-rs GitHub](https://github.com/lumeohq/onvif-rs) — schema crate structure, yaserde 0.7 pin, type coverage
- [hawkeye217 FOV detection gist](https://gist.github.com/hawkeye217/152a1d4ba80760dac95d46e143d37112) — Frigate TranslationSpaceFov check logic
- [Frigate autotracking documentation](https://docs.frigate.video/configuration/autotracking/) — PTZ camera requirements
- [onvif-rs issue #114](https://github.com/lumeohq/onvif-rs/issues/114) — nonce/Created format validation against strict cameras
- [python-zeep issue #1205](https://github.com/mvantellingen/python-zeep/issues/1205) — ter: namespace fault error in the wild
- [xsd-parser docs.rs v1.5.2](https://docs.rs/xsd-parser/latest/xsd_parser/) — fallback type generation approach

### Tertiary (LOW confidence)
- WebSearch results for tokio 1.51 LTS designation, uuid 1.23.0, thiserror 2.0.18 — version searches only, not verified against official release pages
- [ONVIF Profile S deprecation announcement](https://www.onvif.org/?post_type=pressrelease&p=8621) — March 2027 conformance end date

---
*Research completed: 2026-04-05*
*Ready for roadmap: yes*
