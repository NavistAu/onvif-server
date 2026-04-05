# Feature Research

**Domain:** ONVIF Device Server Library (Rust crate)
**Researched:** 2026-04-05
**Confidence:** HIGH (ONVIF spec documents, Frigate source, client source code)

---

## Feature Landscape

### Table Stakes (Clients Break Without These)

These are operations that real ONVIF clients call unconditionally. A missing or fault-returning implementation will cause client initialization failures, not graceful degradation.

#### Device Management Service

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `GetSystemDateAndTime` | First call by every client; used to compute WS-Security digest nonce offsets. Per ONVIF Core spec, must be accessible without authentication. | LOW | Auth-exempt. Return UTC time + timezone. Mandatory for any ONVIF device. |
| `GetCapabilities` | Legacy discovery method; python-onvif-zeep calls this to find XAddrs (service URLs) for all services. All NVRs and ODM call this. | LOW | Returns Media, PTZ, Imaging, Events XAddrs. Pre-`GetServices` fallback. |
| `GetServices` | Newer capability discovery replacing `GetCapabilities`; provides richer feature info. HA and modern clients prefer this. | LOW | Must include capability XML per service. Supersedes GetCapabilities but both required for compat. |
| `GetDeviceInformation` | Returns manufacturer, model, firmware, serial, hardware ID. Every NVR/VMS shows this in camera info. | LOW | All values configurable by the library consumer via `DeviceInfo` struct. |
| `GetScopes` | Used by WS-Discovery and some clients to identify device type. Must return `onvif://www.onvif.org/type/video_encoder`, `onvif://www.onvif.org/name/…`, etc. | LOW | Non-empty scope list mandatory per spec. Clients filter discovery by scope values. |
| WS-Security authentication (UsernameToken digest) | Required for all authenticated operations. Clients compute `B64ENCODE(SHA1(nonce+date+password))`. | MEDIUM | Built on `soap-server`. Time sync via `GetSystemDateAndTime` is prerequisite. |

#### Media Service (Profile S — live streaming)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `GetProfiles` | Returns media profiles (combinations of video source + encoder config). Called at startup by every client. Frigate, HA, ODM, all NVRs call this. | LOW | Must return at least one ready-to-use profile. Profile token used in all subsequent calls. |
| `GetStreamUri` | Returns RTSP URL for live streaming. Core of Profile S. Every client needs this to actually display video. | LOW | URL format: `rtsp://{host}:{port}/…`. The library provides the URL from consumer config; no RTSP server is bundled. |
| `GetVideoSources` | Lists video sources (sensors). Frigate calls this during init to get source tokens for imaging. Required for building complete profiles. | LOW | Can return a single virtual video source. |
| `GetVideoSourceConfigurations` | Lists video source configurations referencing the video sources. | LOW | Clients enumerate these to understand what configurations are available. |
| `GetVideoEncoderConfigurations` | Lists encoder configurations. NVRs examine these to verify H.264 support. | LOW | Needs to reflect the actual encoding the downstream RTSP stream provides. |
| `GetSnapshotUri` (conditional) | Widely called by Home Assistant and NVRs for thumbnails. Conditional: only required if device has snapshot capability. | LOW | Advertise capability only if consumer provides a snapshot URL. Return SOAP fault otherwise — clients handle this gracefully. |

#### PTZ Service

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `GetNodes` | Clients discover PTZ capability by reading the PTZ node. Contains all supported coordinate spaces. **Frigate specifically checks for `TranslationSpaceFov` here.** | MEDIUM | Must advertise `RelativePanTiltTranslationSpace` with `TranslationSpaceFov` URI for Frigate compatibility. |
| `GetConfigurations` | Clients call this to enumerate all PTZ configurations. Milestone uses first result if `GetCompatibleConfiguration` fails. | LOW | Each configuration references a node token. |
| `GetConfiguration` / `GetConfigurationOptions` | Frigate calls `GetConfigurationOptions` at startup to verify FOV space support. Milestone/ODM call to inspect limits. | MEDIUM | Options must include `Spaces.RelativePanTiltTranslationSpace` with `TranslationSpaceFov`. |
| `GetServiceCapabilities` (PTZ) | Frigate checks `MoveStatus: true` here before enabling autotracking. Missing this → autotracking disabled. | LOW | Must return `Capabilities MoveStatus="true"`. Consumer may set `StatusPosition` separately. |
| `RelativeMove` | Core PTZ command for Frigate autotracking. Per-frame command during tracking. | MEDIUM | Requires correct space handling. If no space specified, use node default. |
| `ContinuousMove` | Used by Frigate UI PTZ controls and most other clients for manual PTZ. | LOW | Accept `Velocity` (pan, tilt, zoom). Timeout optional. |
| `Stop` | Stops active PTZ movement. Called after `ContinuousMove` and after autotracking ends. | LOW | Accept `PanTilt` and `Zoom` booleans; stop both if unspecified. |
| `GetStatus` | Frigate polls this at high frequency during autotracking to detect MOVING/IDLE transition. | LOW | Must return `MoveStatus.PanTilt` and `MoveStatus.Zoom` as `IDLE`/`MOVING`. Position field optional. |
| `GetPresets` | Frigate calls at startup to find home preset. Used for return-to-home at end of tracking session. | LOW | Return list of preset tokens + names. Empty list valid if no presets configured. |
| `GotoPreset` | Frigate calls this to return camera to home position after tracking. | LOW | Accept preset token, execute move. Speed optional. |

---

### Differentiators (Competitive Advantage for a Rust Crate)

These features distinguish this library from DIY integrations and existing client-only crates.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Trait-based service API with `not_implemented()` defaults | Library consumers only implement the subset they support. All other operations return spec-compliant SOAP faults automatically. No boilerplate. | MEDIUM | This is the core library API design. Default trait methods return `Err(OnvifError::NotImplemented)` which maps to `env:Sender` SOAP fault. |
| Bundled official WSDLs/XSDs | Consumer does not need to source ONVIF spec files. `wsdl_url` responses point to the embedded files. | LOW | Bundle `devicemgmt.wsdl`, `media.wsdl`, `ptz.wsdl`, `imaging.wsdl`, `events.wsdl`, `onvif.xsd`, `types.xsd`. HTTP-served by the SOAP layer. |
| TranslationSpaceFov advertisement built-in | Frigate autotracking requires advertising this space in `GetNodes` and `GetConfigurationOptions`. Getting this wrong silently breaks Frigate. The library handles the correct XML structure automatically. | MEDIUM | Pre-built structs in PTZ node builder for FOV space ranges. Consumer provides min/max values. |
| MoveStatus capability advertisement | `GetServiceCapabilities` must return `MoveStatus="true"` for Frigate polling to work. Library handles this with correct XML attribute name (not just `true`/`false` bool). | LOW | Wraps the exact XML structure Frigate's `python-onvif-zeep` expects. |
| Auth-exempt operation registry | `GetSystemDateAndTime` must bypass WS-Security before credentials are known. Library registers exempt operations automatically without consumer needing to know the rule. | LOW | Delegates to `soap-server` auth exemption mechanism. Extensible list if spec adds more exempt ops. |
| WS-Discovery (`discovery` feature flag) | Auto-discovery by Home Assistant, Synology Surveillance Station, AgentDVR, and most NVRs. Not needed for Frigate (direct URL) but required for "just plug in" compatibility. | HIGH | UDP multicast on `239.255.255.250:3702`. ProbeMatch response with XAddrs. Behind `#[cfg(feature = "discovery")]`. |
| Type-safe builder pattern | Single `OnvifServer::builder()` wires all services, configures auth, sets port. No raw SOAP knowledge required. | MEDIUM | Builder validates required fields (device info, at least Device service). Compile error for missing required fields using typestate pattern. |
| Frigate compatibility test suite | Integration tests replaying the exact call sequence Frigate's autotracker makes. Prevents regressions when changing PTZ implementation. | MEDIUM | Tests in `tests/frigate_compat.rs`. Runs against an in-process virtual PTZ camera example. |

---

### Anti-Features (Deliberately NOT in v1)

| Anti-Feature | Why Requested | Why Avoid | Alternative |
|--------------|---------------|-----------|-------------|
| RTSP server / video streaming | "Complete camera" feel. Users want one binary. | Scope explosion. RTSP is a separate protocol stack (H.264/H.265, RTP, SDP). Out of scope entirely — this is a SOAP/ONVIF layer only. Consumer provides RTSP URL. | Consumer runs their own RTSP server (e.g., `mediamtx`, `rtsp-simple-server`). `GetStreamUri` returns that URL. |
| ONVIF client functionality | Same crate = less dependencies. | Clients need different abstractions (deserialization-first, not server-side). Mixing would pollute the API. `onvif-rs` already exists for this. | Use `lumeohq/onvif-rs` for client use cases. |
| Profile G (recording/search) | "Full ONVIF compliance" requests. | Recording control and search services are a completely separate domain (timelines, recording jobs, `FindRecordings`, `GetRecordingInformation`). No downstream use case for Fovealink or PTZ proxies. | Implement in a separate `onvif-recording` crate if demand emerges. |
| ONVIF conformance certification | "Official compliance" badge. | Conformance testing requires ONVIF membership (fees) and hardware test bench. Adds no value for a Rust library — interoperability matters, not the badge. | Test against real clients (Frigate, ODM, HA) instead. |
| Multi-service-instance routing | Advanced use: multiple cameras on one port with different auth. | Significant complexity in the routing layer. No current downstream use case. | Consumer runs one `OnvifServer` per camera/port. |
| ONVIF Media2 service (Profile T) | H.265 and richer metadata. | Profile T requires Media2 service, metadata streaming, and H.265 encoder configurations. No current client need (Frigate, HA use Media/Profile S). | Add in v2 if H.265 demand emerges. Design traits to be extensible. |
| PTZ auxiliary commands (SendAuxiliaryCommand) | Complete PTZ spec coverage. | Rarely implemented, rarely called, vendor-specific payloads. | Default `not_implemented()` fault is sufficient. |
| User management (CreateUser, DeleteUser) | Full device management. | Security-sensitive operations. For a virtual device/proxy, auth is configured at build time, not via ONVIF user management commands. | Configure credentials in builder. |

---

## Per-Service Operation Coverage Expectations

### Device Management Service (devicemgmt.wsdl, ~40 ops)

| Operation | Client Usage | v1 Required | Notes |
|-----------|-------------|-------------|-------|
| `GetSystemDateAndTime` | Every client, first call | YES | Auth-exempt |
| `GetCapabilities` | python-onvif-zeep, legacy clients | YES | Returns XAddrs for all services |
| `GetServices` | HA, modern clients | YES | Richer than GetCapabilities |
| `GetDeviceInformation` | Every client/NVR | YES | Consumer-provided values |
| `GetScopes` | Discovery, some clients | YES | Must return non-empty list |
| `GetHostname` | ODM, NVRs | MEDIUM | Fault acceptable; include if easy |
| `GetNetworkInterfaces` | ODM, some NVRs | LOW | Fault acceptable |
| `GetDNS` | ODM | LOW | Fault acceptable |
| `GetNetworkProtocols` | ODM | LOW | Fault acceptable |
| `GetDiscoveryMode` | ODM | LOW | Fault acceptable |
| `GetNTP` | Some NVRs | LOW | Fault acceptable |
| `GetUsers` | ODM, security tools | LOW | Fault acceptable |
| `SystemReboot` | Management tools | LOW | Fault acceptable |
| `GetSystemLog` | Diagnostics | LOW | Fault acceptable |

### Media Service (media.wsdl, ~25 ops)

| Operation | Client Usage | v1 Required | Notes |
|-----------|-------------|-------------|-------|
| `GetProfiles` | Every client | YES | Must have at least one profile |
| `GetStreamUri` | Every client | YES | Consumer provides RTSP URL |
| `GetVideoSources` | Frigate, HA | YES | Required for imaging service link |
| `GetVideoSourceConfigurations` | NVRs, ODM | YES | Links source to profile |
| `GetVideoEncoderConfigurations` | NVRs (check codec) | YES | Declare H.264 or consumer codec |
| `GetSnapshotUri` | HA, Synology, some NVRs | MEDIUM | Conditional on consumer advertising SnapshotUri capability |
| `GetAudioSources` | Audio-capable clients | LOW | Fault if no audio |
| `GetAudioEncoderConfigurations` | Audio clients | LOW | Fault if no audio |
| `CreateProfile` / `DeleteProfile` | Config tools | LOW | Fault acceptable |
| `AddVideoSourceConfiguration` | Config tools | LOW | Fault acceptable |

### PTZ Service (ptz.wsdl, ~15 ops)

| Operation | Client Usage | v1 Required | Notes |
|-----------|-------------|-------------|-------|
| `GetNodes` | Frigate (checks FOV space), all PTZ clients | YES | Must advertise TranslationSpaceFov |
| `GetNode` | Frigate, Milestone | YES | Single node lookup |
| `GetConfigurations` | Milestone, all PTZ clients | YES | Returns list of PTZ configs |
| `GetConfiguration` | Targeted lookup | YES | |
| `GetConfigurationOptions` | Frigate (startup check) | YES | Must include FOV space in options |
| `GetServiceCapabilities` | Frigate (MoveStatus check) | YES | MoveStatus="true" required for autotracking |
| `RelativeMove` | Frigate (per-frame autotrack) | YES | FOV space. Core command. |
| `ContinuousMove` | Frigate UI, all PTZ clients | YES | Manual PTZ control |
| `Stop` | Frigate, all clients | YES | After ContinuousMove |
| `GetStatus` | Frigate (polls during tracking) | YES | Returns IDLE/MOVING |
| `GetPresets` | Frigate (home position) | YES | Empty list OK |
| `GotoPreset` | Frigate (return home), NVRs | YES | |
| `AbsoluteMove` | Frigate (zoom), some clients | MEDIUM | Fault acceptable if not supporting absolute |
| `SetPreset` | Management tools | LOW | Fault acceptable |
| `RemovePreset` | Management tools | LOW | Fault acceptable |
| `GotoHomePosition` | Some clients | LOW | Fault acceptable |

### Imaging Service (imaging.wsdl, ~10 ops)

| Operation | Client Usage | v1 Required | Notes |
|-----------|-------------|-------------|-------|
| `GetImagingSettings` | Frigate (focus/zoom info at startup) | YES | Return minimal settings |
| `GetOptions` | Config tools | LOW | Fault acceptable |
| `SetImagingSettings` | Config tools | LOW | Fault acceptable |
| `GetMoveOptions` | Focus control clients | LOW | Fault acceptable if no focus |
| `Move` (focus) | Frigate (focus control) | MEDIUM | Frigate calls this for focus; fault if not supported |
| `GetStatus` (imaging) | Focus status polling | LOW | Fault acceptable |

### Events Service (events.wsdl, ~10 ops)

| Operation | Client Usage | v1 Required | Notes |
|-----------|-------------|-------------|-------|
| `GetEventProperties` | HA, NVRs (enumerate available events) | MEDIUM | HA calls this. Fault means no events — clients handle gracefully |
| `CreatePullPointSubscription` | HA, NVRs for motion events | MEDIUM | Required if consumer wants to publish motion/alarm events |
| `PullMessages` | HA polls after subscribing | MEDIUM | Must respond to poll with correct envelope even if no events |
| `Unsubscribe` | HA cleanup | MEDIUM | Should clean up subscriptions |

---

## Feature Dependencies

```
WS-Security (authentication)
    └──requires──> GetSystemDateAndTime (auth-exempt, provides clock sync)
                       └──enables──> All authenticated operations

GetCapabilities / GetServices
    └──enables──> Service URL discovery (PTZ, Media, Imaging, Events XAddrs)
                       └──enables──> All per-service operations

PTZ.GetNodes
    └──required-before──> PTZ.GetConfigurations
                              └──required-before──> PTZ.RelativeMove (config token)
                              └──required-before──> PTZ.GetConfigurationOptions
                              └──required-before──> PTZ.GetStatus

Media.GetProfiles
    └──required-before──> Media.GetStreamUri (profile token)
    └──required-before──> PTZ.GetStatus (profile token)
    └──required-before──> PTZ.RelativeMove (profile token)

Media.GetVideoSources
    └──required-before──> Imaging.GetImagingSettings (video source token)

PTZ.GetServiceCapabilities (MoveStatus=true)
    └──enables──> Frigate autotracking (without this, Frigate disables autotrack silently)

PTZ.GetConfigurationOptions (TranslationSpaceFov advertised)
    └──enables──> Frigate RelativeMove in FOV space (wrong space = camera moves incorrectly)

WS-Discovery (feature flag)
    └──enables──> NVR auto-discovery (Synology, HA, AgentDVR)
    └──depends-on──> GetScopes (scopes broadcast in ProbeMatch)
```

### Dependency Notes

- **GetSystemDateAndTime requires no auth:** Clients call this before they can compute any WS-Security digest. If auth is enforced here, the client cannot authenticate any subsequent call.
- **Profile token threads through everything:** The same token from `GetProfiles` is passed to `GetStreamUri`, `RelativeMove`, `GetStatus`, `GotoPreset`, and `GetImagingSettings`. A mismatched token causes client errors.
- **PTZ node token threads through PTZ:** Node token from `GetNodes` is embedded in configurations from `GetConfigurations`. Config token from `GetConfigurations` is in `GetConfigurationOptions` request, `RelativeMove`, `Stop`, `GetStatus`.
- **TranslationSpaceFov must match between GetNodes and GetConfigurationOptions:** Frigate validates capability in both places. Inconsistency between node advertisement and configuration options causes silent failures.
- **MoveStatus capability enables Frigate poll loop:** Without `GetServiceCapabilities` returning `MoveStatus="true"`, Frigate's autotracker never enters the status-polling loop and will not wait for PTZ to settle before commanding the next move.

---

## MVP Definition

### Launch With (v1)

Minimum to validate against Frigate autotracker and ONVIF Device Manager.

- [ ] Device management service: `GetSystemDateAndTime` (auth-exempt), `GetCapabilities`, `GetServices`, `GetDeviceInformation`, `GetScopes` — basic identity and discovery
- [ ] Media service: `GetProfiles`, `GetStreamUri`, `GetVideoSources`, `GetVideoSourceConfigurations`, `GetVideoEncoderConfigurations` — required for any streaming client
- [ ] PTZ service: `GetNodes` (with TranslationSpaceFov), `GetConfigurations`, `GetConfiguration`, `GetConfigurationOptions` (with FOV space), `GetServiceCapabilities` (MoveStatus=true), `RelativeMove`, `ContinuousMove`, `Stop`, `GetStatus`, `GetPresets`, `GotoPreset` — full Frigate autotracking surface
- [ ] Imaging service: `GetImagingSettings` — Frigate reads this at startup
- [ ] WS-Security UsernameToken digest authentication for all non-exempt operations
- [ ] Builder pattern: `OnvifServer::builder()` with service registration and `auth()`, `port()`, `build()` and `run()`
- [ ] SOAP fault for all unimplemented operations (default trait method returns `not_implemented()` fault)
- [ ] `virtual_ptz` example demonstrating minimal consumer implementation

### Add After Validation (v1.x)

Features to add once Frigate compatibility is confirmed.

- [ ] `GetSnapshotUri` — trigger: Home Assistant integration testing
- [ ] `GetHostname`, `GetNetworkInterfaces`, `GetDNS` — trigger: ODM compatibility testing reveals faults are not acceptable
- [ ] Events service (`CreatePullPointSubscription`, `PullMessages`) — trigger: Home Assistant motion detection use case
- [ ] `AbsoluteMove` — trigger: Frigate zoom control testing

### Future Consideration (v2+)

Features to defer until downstream consumer needs drive them.

- [ ] WS-Discovery (`discovery` feature flag) — defer until an NVR auto-discovery use case exists; direct URL works for all current targets
- [ ] Media2 service (Profile T / H.265) — defer until a client requires H.265 that won't fall back to H.264
- [ ] Profile G (recording services) — defer until recording use case exists; requires entirely different service surface

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| GetSystemDateAndTime (auth-exempt) | HIGH | LOW | P1 |
| GetCapabilities + GetServices | HIGH | LOW | P1 |
| GetProfiles + GetStreamUri | HIGH | LOW | P1 |
| PTZ.GetNodes (TranslationSpaceFov) | HIGH | MEDIUM | P1 |
| PTZ.GetServiceCapabilities (MoveStatus) | HIGH | LOW | P1 |
| PTZ.RelativeMove | HIGH | MEDIUM | P1 |
| PTZ.GetStatus (IDLE/MOVING) | HIGH | LOW | P1 |
| PTZ.ContinuousMove + Stop | HIGH | LOW | P1 |
| PTZ.GetPresets + GotoPreset | HIGH | LOW | P1 |
| WS-Security authentication | HIGH | MEDIUM | P1 |
| Imaging.GetImagingSettings | MEDIUM | LOW | P1 |
| Builder pattern API | HIGH | MEDIUM | P1 |
| GetSnapshotUri | MEDIUM | LOW | P2 |
| Events (PullPoint) | MEDIUM | HIGH | P2 |
| GetHostname / network ops | LOW | LOW | P2 |
| AbsoluteMove | MEDIUM | LOW | P2 |
| WS-Discovery | MEDIUM | HIGH | P3 |
| Media2 / Profile T | LOW | HIGH | P3 |
| Profile G (recording) | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for launch (Frigate + ODM compatibility)
- P2: Should have — add when P1 is validated
- P3: Nice to have — future consideration

---

## Competitor Feature Analysis

| Feature | onvif-rs (lumeohq) | onvif-camera-mock (C/gsoap) | python-onvif-zeep (client only) | Our Approach |
|---------|--------------------|-----------------------------|----------------------------------|--------------|
| Rust language | YES (client only) | NO | NO | YES (server) |
| Server-side API | NO — client only | YES — C, not ergonomic | NO | YES — trait-based |
| Type generation | YES (xsd-parser-rs) | NO — hand-coded | NO | Reuse onvif-rs types or regenerate |
| TranslationSpaceFov | N/A (client parses it) | Unclear | N/A | Explicit builder helpers |
| Default SOAP faults | N/A | Unclear | N/A | Built-in via trait defaults |
| WS-Discovery | YES (client) | Separate binary (wsdd) | YES (client) | Feature-gated |
| Frigate compat tests | NO | NO | N/A | YES — test suite |

---

## Sources

- [Frigate Autotracking Documentation](https://docs.frigate.video/configuration/autotracking/) — requirements for PTZ camera compatibility
- [Frigate FOV compatibility check gist (hawkeye217)](https://gist.github.com/hawkeye217/152a1d4ba80760dac95d46e143d37112) — exact four conditions Frigate checks
- [python-onvif-zeep client.py initialization sequence](https://github.com/FalkTannhaeuser/python-onvif-zeep/blob/zeep/onvif/client.py) — startup call order
- [ONVIF Profiles overview](https://www.onvif.org/profiles/) — Profile S, T, G summary
- [ONVIF Profile S deprecation announcement (Oct 2025)](https://www.onvif.org/?post_type=pressrelease&p=8621) — Profile T is the replacement; Profile S conformance ends March 2027
- [ONVIF Core Specification v25.12](https://www.onvif.org/specs/core/ONVIF-Core-Specification.pdf) — access policy, auth exemptions, GetScopes
- [ONVIF PTZ Service Specification v25.12](https://www.onvif.org/specs/srv/ptz/ONVIF-PTZ-Service-Spec.pdf) — coordinate spaces, GetNodes, GetStatus
- [ONVIF Media Service Specification v24.12](https://www.onvif.org/specs/srv/media/ONVIF-Media-Service-Spec.pdf) — GetProfiles, GetStreamUri, GetSnapshotUri
- [Home Assistant virtual ONVIF add-on (tocje)](https://github.com/tocje/ha-virtual-onvif) — what HA needs from a virtual ONVIF device
- [ONVIF Device Feature Discovery Specification v21.12](https://www.onvif.org/wp-content/uploads/2022/07/ONVIF_Device_Feature_Discovery_Specification_21.12.pdf) — GetServices vs GetCapabilities usage patterns

---
*Feature research for: Rust ONVIF Device Server crate*
*Researched: 2026-04-05*
