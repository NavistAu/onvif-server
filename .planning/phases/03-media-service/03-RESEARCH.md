# Phase 3: Media Service - Research

**Researched:** 2026-04-05
**Domain:** ONVIF Profile S Media Service — GetProfiles, GetStreamUri, GetSnapshotUri, GetVideoSources, GetVideoSourceConfigurations, GetVideoEncoderConfigurations
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Porting operation following best prior art (onvif-rs, python-onvif-zeep, ONVIF spec) — same as Phases 1-2
- Follow established patterns from the research — don't reinvent where prior art already defines the right answer
- Hand-written types (not codegen) — same approach as Phase 1-2
- DeviceServiceHandler dispatch pattern (extract_local_name + match) — reuse for MediaServiceHandler
- Token constants already defined: PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV
- Builder already accepts `.media_service(impl)` — just needs wiring in `run()` like Device was wired in Phase 2

### Claude's Discretion
- MediaServiceHandler implementation following the DeviceServiceHandler pattern from Phase 2
- Profile XML structure: Must include PTZConfiguration with DefaultContinuousPanTiltVelocitySpace set to TRANSLATION_SPACE_FOV URI — critical Frigate pitfall #2 from research
- MediaService trait signatures: typed returns for GetProfiles, GetStreamUri, GetVideoSources, etc.
- Type expansion for Media-specific structures (Profile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration)
- Multi-service router merging in run() — adding Media service alongside Device service
- All technical decisions follow research findings, DESIGN.md, and the Phase 2 DeviceServiceHandler as pattern reference

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MEDIA-01 | User can call GetProfiles and receive at least one media profile with video source, encoder, and PTZ configuration references | Profile XML structure documented; PTZConfiguration fields confirmed from onvif.xsd |
| MEDIA-02 | User can call GetStreamUri with a profile token and receive an RTSP URL as configured by the consumer | GetStreamUri request/response structure confirmed from media.wsdl; MediaUri type fields documented |
| MEDIA-03 | User can call GetVideoSources and receive video source descriptions with resolution and frame rate | VideoSource type confirmed from onvif.xsd; token = VIDEO_SOURCE_TOKEN |
| MEDIA-04 | User can call GetVideoSourceConfigurations and receive configurations linking video sources to profiles | VideoSourceConfiguration type confirmed from onvif.xsd; requires SourceToken + Bounds |
| MEDIA-05 | User can call GetVideoEncoderConfigurations and receive encoder settings (codec, resolution, bitrate) | VideoEncoderConfiguration type confirmed from onvif.xsd; Multicast element is required by spec |
| MEDIA-06 | User can call GetSnapshotUri with a profile token and receive a snapshot URL as configured by the consumer | GetSnapshotUri request/response identical to GetStreamUri pattern; MediaUri response type identical |
</phase_requirements>

## Summary

Phase 3 implements the ONVIF Profile S Media Service — a read-only set of six operations that describe media capabilities and provide stream/snapshot URIs. The implementation is a direct port of the proven DeviceServiceHandler pattern from Phase 2: one `MediaServiceHandler` struct implements `SoapHandler`, dispatches via `extract_local_name + match`, and delegates to a `MediaService` trait for consumer-provided values (stream URI, snapshot URI). Four of six operations return static data built from token constants; only `GetStreamUri` and `GetSnapshotUri` require consumer delegation.

The critical correctness constraint for Frigate compatibility is the `GetProfiles` response: the `PTZConfiguration` element inside the profile MUST include `tt:DefaultContinuousPanTiltVelocitySpace` set to the `TRANSLATION_SPACE_FOV` URI constant. Without this element, Frigate silently disables PTZ autotracking. All other fields follow directly from the ONVIF spec as read from the bundled `onvif.xsd` and `media.wsdl`.

The Media service mounts at `/onvif/media_service` and is merged into the axum router alongside the Device service via `Router::merge()`. The `run()` method in `server.rs` needs a second `soap_server::ServerBuilder` block for the media WSDL.

**Primary recommendation:** Mirror DeviceServiceHandler exactly — same file layout, same dispatch pattern, same XML string-building approach. Expand `MediaService` trait to typed returns, add Rust type stubs for the five new types, implement six handler methods, mount via `Router::merge()`, add integration test file mirroring `tests/device_management.rs`.

---

## Standard Stack

### Core (already in Cargo.toml — no new dependencies needed)

| Library | Version | Purpose | Already Present |
|---------|---------|---------|----------------|
| `quick-xml` | 0.39 | Parse incoming SOAP body to extract operation name | Yes — `extract_local_name` reuse |
| `bytes` | 1 | `Bytes` in/out for `SoapHandler::handle` | Yes |
| `async-trait` | 0.1 | `#[async_trait]` on `MediaService` trait | Yes |
| `soap-server` | path dep | `SoapHandler`, `SoapFault`, `ServerBuilder` | Yes |
| `axum` | 0.8 | `Router::merge()` for multi-service routing | Yes |

**No new dependencies required.** All libraries already present.

### New Rust Types Needed (hand-written stubs in `src/generated/types.rs`)

| Type | Fields | Source |
|------|--------|--------|
| `MediaProfile` | `token: String`, `name: String`, `video_source_cfg_token: String`, `video_encoder_cfg_token: String`, `ptz_cfg_token: String` | onvif.xsd `Profile` type |
| `VideoSource` | `token: String`, `framerate: f32`, `width: i32`, `height: i32` | onvif.xsd `VideoSource` type |
| `VideoSourceConfiguration` | `token: String`, `name: String`, `source_token: String`, `bounds_x: i32`, `bounds_y: i32`, `bounds_width: i32`, `bounds_height: i32` | onvif.xsd `VideoSourceConfiguration` type |
| `VideoEncoderConfiguration` | `token: String`, `name: String`, `encoding: String`, `width: i32`, `height: i32`, `framerate: i32`, `bitrate: i32` | onvif.xsd `VideoEncoderConfiguration` type |

Note: `MediaProfile` is a simplified Rust struct — it does not need to model all optional fields from the XSD. The handler renders the XML directly from constants and these fields.

---

## Architecture Patterns

### Recommended File Structure

```
src/
├── service/
│   ├── mod.rs          # add: pub mod media;
│   ├── device.rs       # existing — no changes
│   └── media.rs        # NEW — MediaServiceHandler
├── traits/
│   └── media.rs        # MODIFY — typed return signatures
├── generated/
│   └── types.rs        # MODIFY — add 4 new type stubs
├── server.rs           # MODIFY — wire MediaServiceHandler in run()
└── lib.rs              # MODIFY — pub use new types + MediaServiceHandler
tests/
└── media_service.rs    # NEW — integration tests for MEDIA-01..06
```

### Pattern 1: MediaServiceHandler mirrors DeviceServiceHandler

**What:** One struct implementing `SoapHandler`, dispatching by operation name, calling trait methods or returning static XML.
**When to use:** Every ONVIF service handler in this codebase.

```rust
// src/service/media.rs — mirrors src/service/device.rs exactly
use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};
use quick_xml::NsReader;
use quick_xml::events::Event;

use crate::error::OnvifError;
use crate::traits::MediaService;
use crate::constants::{
    PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV
};

pub struct MediaServiceHandler {
    pub(crate) svc: Arc<dyn MediaService>,
    pub(crate) xaddr: String,
}

#[async_trait]
impl SoapHandler for MediaServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetProfiles"                    => self.handle_get_profiles().await,
            "GetStreamUri"                   => self.handle_get_stream_uri(&body).await,
            "GetSnapshotUri"                 => self.handle_get_snapshot_uri(&body).await,
            "GetVideoSources"                => self.handle_get_video_sources().await,
            "GetVideoSourceConfigurations"   => self.handle_get_video_source_configurations().await,
            "GetVideoEncoderConfigurations"  => self.handle_get_video_encoder_configurations().await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}
```

`extract_local_name` is duplicated (not shared) — same as how `device.rs` has its own copy. This is the established pattern.

### Pattern 2: GetProfiles XML with PTZConfiguration (critical Frigate path)

**What:** The GetProfiles response includes a full PTZ configuration reference inside the profile. The `DefaultContinuousPanTiltVelocitySpace` field is what Frigate checks to determine if PTZ autotracking is enabled.

**When to use:** Always. Static — built from constants, no trait delegation needed.

```rust
async fn handle_get_profiles(&self) -> Result<Bytes, SoapFault> {
    let xml = format!(
        r#"<trt:GetProfilesResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:Profiles token="{profile_token}" fixed="true">
    <tt:Name>MainProfile</tt:Name>
    <tt:VideoSourceConfiguration token="{vs_cfg_token}">
      <tt:Name>VideoSourceConfig</tt:Name>
      <tt:UseCount>1</tt:UseCount>
      <tt:SourceToken>{vs_token}</tt:SourceToken>
      <tt:Bounds x="0" y="0" width="1920" height="1080"/>
    </tt:VideoSourceConfiguration>
    <tt:VideoEncoderConfiguration token="{ve_cfg_token}">
      <tt:Name>VideoEncoderConfig</tt:Name>
      <tt:UseCount>1</tt:UseCount>
      <tt:Encoding>H264</tt:Encoding>
      <tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution>
      <tt:Quality>5</tt:Quality>
      <tt:RateControl>
        <tt:FrameRateLimit>30</tt:FrameRateLimit>
        <tt:EncodingInterval>1</tt:EncodingInterval>
        <tt:BitrateLimit>4096</tt:BitrateLimit>
      </tt:RateControl>
      <tt:Multicast>
        <tt:Address><tt:Type>IPv4</tt:Type><tt:IPv4Address>0.0.0.0</tt:IPv4Address></tt:Address>
        <tt:Port>0</tt:Port>
        <tt:TTL>0</tt:TTL>
        <tt:AutoStart>false</tt:AutoStart>
      </tt:Multicast>
      <tt:SessionTimeout>PT10S</tt:SessionTimeout>
    </tt:VideoEncoderConfiguration>
    <tt:PTZConfiguration token="{ptz_cfg_token}">
      <tt:Name>PTZConfig</tt:Name>
      <tt:UseCount>1</tt:UseCount>
      <tt:NodeToken>{ptz_node_token}</tt:NodeToken>
      <tt:DefaultContinuousPanTiltVelocitySpace>{translation_space_fov}</tt:DefaultContinuousPanTiltVelocitySpace>
    </tt:PTZConfiguration>
  </trt:Profiles>
</trt:GetProfilesResponse>"#,
        profile_token = PROFILE_TOKEN,
        vs_cfg_token = VIDEO_SOURCE_TOKEN,
        vs_token = VIDEO_SOURCE_TOKEN,
        ve_cfg_token = "video_enc_0",
        ptz_cfg_token = PTZ_CONFIG_TOKEN,
        ptz_node_token = crate::constants::PTZ_NODE_TOKEN,
        translation_space_fov = TRANSLATION_SPACE_FOV,
    );
    Ok(Bytes::from(xml))
}
```

**Key observation:** The Media service WSDL namespace prefix is `trt:` (not `tds:`). This is confirmed by the media.wsdl WSDL port binding section and standard onvif-rs usage. Device used `tds:`.

### Pattern 3: GetStreamUri and GetSnapshotUri — parse ProfileToken from body

**What:** Both operations include a `ProfileToken` in the request body. The handler must extract it before delegating to the trait.
**When to use:** Whenever a request carries input parameters.

```rust
async fn handle_get_stream_uri(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
    // Extract ProfileToken text from XML body using quick-xml
    let profile_token = extract_text_element(body, "ProfileToken")?;
    let uri = self.svc.get_stream_uri(&profile_token).await
        .map_err(|e| e.into_soap_fault())?;
    let xml = format!(
        r#"<trt:GetStreamUriResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:MediaUri>
    <tt:Uri>{uri}</tt:Uri>
    <tt:InvalidAfterConnect>false</tt:InvalidAfterConnect>
    <tt:InvalidAfterReboot>false</tt:InvalidAfterReboot>
    <tt:Timeout>PT0S</tt:Timeout>
  </trt:MediaUri>
</trt:GetStreamUriResponse>"#,
        uri = uri
    );
    Ok(Bytes::from(xml))
}
```

`extract_text_element` is a new private helper in `media.rs` that walks the XML to find the first element with the given local name and returns its text content.

### Pattern 4: Router::merge() for multi-service in run()

**What:** The existing `run()` creates one `SoapService` for device management. Media service needs a second `SoapService` on a different path, then merged via `Router::merge()`.

```rust
// In OnvifServer::run() — add after the device soap_svc block:
let media_svc = self.media_service
    .ok_or("media_service is required to call run()")?;

let media_xaddr = format!("http://0.0.0.0:{}/onvif/media_service", self.port);
let media_handler = MediaServiceHandler { svc: media_svc, xaddr: media_xaddr };

let media_soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
        include_bytes!("../wsdl/media.wsdl").to_vec(),
        EmbeddedWsdlLoader,
    )
    .path("/onvif/media_service")
    .default_handler(media_handler)
    .auth(move |user: &str| -> Option<String> { /* same closure as device */ })
    .auth_bypass(auth_bypass.into_iter())
    .build()
    .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

let router = device_soap_svc.into_router()
    .merge(media_soap_svc.into_router());
```

**Constraint:** The auth closure captures `username`/`password` by value (they are `Option<String>`). Since both services need the same closure but closures can't be cloned, the username/password must be cloned before building each service. This is the same pattern that would be needed in Phase 4 for PTZ.

### Anti-Patterns to Avoid

- **Wrong namespace prefix:** Media responses use `trt:` not `tds:`. Using `tds:` on Media responses will cause client parse failures.
- **Missing PTZConfiguration in GetProfiles:** Omitting PTZConfiguration or omitting `DefaultContinuousPanTiltVelocitySpace` within it causes Frigate to silently disable PTZ autotracking. Always include it.
- **Missing Multicast in VideoEncoderConfiguration:** The XSD marks `Multicast` as required (no `minOccurs="0"`). Omit it and spec-strict clients reject the response.
- **Missing SessionTimeout in VideoEncoderConfiguration:** Also required (no `minOccurs="0"`) — must be an ISO 8601 duration like `PT10S`.
- **Returning PTZ_NODE_TOKEN in profile PTZConfiguration token attribute vs NodeToken element:** The `token` attribute on `<tt:PTZConfiguration>` is `PTZ_CONFIG_TOKEN`; the `<tt:NodeToken>` child element is `PTZ_NODE_TOKEN`. These are different constants.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| XML namespace handling | Custom namespace tracker | `quick_xml::NsReader` | Already proven in `extract_local_name` |
| SOAP fault serialization | Custom fault builder | `OnvifError::into_soap_fault()` | Already defined in `error.rs` |
| ProfileToken extraction | Manual byte scan | `quick_xml` event loop helper | `NsReader` handles namespace-prefixed elements cleanly |

**Key insight:** The format-string XML approach from Phase 2 is deliberate — it is simpler, more readable, and less error-prone than building an XML tree for these small fixed responses. Stick with it.

---

## Common Pitfalls

### Pitfall 1: Wrong WSDL namespace prefix (trt vs tds)

**What goes wrong:** Handler builds XML with `tds:GetProfilesResponse` instead of `trt:GetProfilesResponse`. Client XML parsers see the wrong namespace and fail or return no data.
**Why it happens:** Copying from the DeviceServiceHandler without changing the namespace prefix.
**How to avoid:** Media service uses `http://www.onvif.org/ver10/media/wsdl` with prefix `trt`. Check media.wsdl binding section: `<wsdl:port name="MediaPort" binding="trt:MediaBinding">`.
**Warning signs:** Client shows empty profile list despite server returning HTTP 200.

### Pitfall 2: Missing DefaultContinuousPanTiltVelocitySpace (Frigate silently breaks)

**What goes wrong:** GetProfiles response includes a PTZConfiguration element but omits `DefaultContinuousPanTiltVelocitySpace`. Frigate's autotracking code checks for this field and disables PTZ if absent.
**Why it happens:** The field is optional in the ONVIF XSD (`minOccurs="0"`), so it's easy to omit when building minimal responses.
**How to avoid:** Always include `<tt:DefaultContinuousPanTiltVelocitySpace>` set to `TRANSLATION_SPACE_FOV` in every GetProfiles PTZConfiguration response.
**Warning signs:** Frigate autotracking menu shows PTZ disabled even though camera is configured.

### Pitfall 3: Multicast/SessionTimeout required fields omitted

**What goes wrong:** GetVideoEncoderConfigurations (and the encoder config embedded in GetProfiles) omits `Multicast` or `SessionTimeout`. Spec-strict clients reject the response.
**Why it happens:** Both fields look optional conceptually (we don't support multicast) but are XSD-required.
**How to avoid:** Always emit a zeroed-out Multicast block (`<tt:Address>...<tt:IPv4Address>0.0.0.0</tt:IPv4Address>`, Port 0, TTL 0, AutoStart false) and `<tt:SessionTimeout>PT10S</tt:SessionTimeout>`.
**Warning signs:** ONVIF Device Manager shows "invalid response" for media operations.

### Pitfall 4: auth closure can't be shared (ownership)

**What goes wrong:** Trying to build two `SoapService` instances with the same auth closure results in a move-after-use compile error.
**Why it happens:** The closure captures `username` and `password` by value; Rust moves them on first use.
**How to avoid:** Clone `username` and `password` before building each `SoapService` block. Each closure owns its own copy.

### Pitfall 5: media_service optional vs required in run()

**What goes wrong:** `run()` panics or returns an error when media_service is `None` but user forgot to register one.
**Why it happens:** `media_service` field is `Option<Arc<dyn MediaService>>`.
**How to avoid:** Provide a clear error message (same pattern as `device_service`). Consider whether media_service should be required at `build()` time or only at `run()` time. Current pattern: required at `run()`, optional at `build()`.

### Pitfall 6: ProfileToken not parsed from GetStreamUri body

**What goes wrong:** Handler ignores the ProfileToken in the request and always returns the same URI regardless of which profile was requested. Fine for single-profile devices, but the trait receives no context.
**Why it happens:** GetStreamUri request has a `StreamSetup` and a `ProfileToken` element in the body — unlike GetProfiles which has no parameters.
**How to avoid:** Parse the `ProfileToken` element from the body using a `extract_text_element` helper before calling the trait method. Pass the token to `get_stream_uri(&str)` and `get_snapshot_uri(&str)`.

---

## Code Examples

### Confirmed Types from onvif.xsd

**VideoSource** (token attribute on DeviceEntity base):
```xml
<trt:VideoSources token="video_src_0">
  <tt:Framerate>30</tt:Framerate>
  <tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution>
</trt:VideoSources>
```

**VideoSourceConfiguration** (extends ConfigurationEntity — has Name, UseCount, token attr):
```xml
<trt:Configurations token="video_src_0">
  <tt:Name>VideoSourceConfig</tt:Name>
  <tt:UseCount>1</tt:UseCount>
  <tt:SourceToken>video_src_0</tt:SourceToken>
  <tt:Bounds x="0" y="0" width="1920" height="1080"/>
</trt:Configurations>
```

**VideoEncoderConfiguration** (required fields: Encoding, Resolution, Quality, Multicast, SessionTimeout):
```xml
<trt:Configurations token="video_enc_0">
  <tt:Name>VideoEncoderConfig</tt:Name>
  <tt:UseCount>1</tt:UseCount>
  <tt:Encoding>H264</tt:Encoding>
  <tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution>
  <tt:Quality>5</tt:Quality>
  <tt:RateControl>
    <tt:FrameRateLimit>30</tt:FrameRateLimit>
    <tt:EncodingInterval>1</tt:EncodingInterval>
    <tt:BitrateLimit>4096</tt:BitrateLimit>
  </tt:RateControl>
  <tt:Multicast>
    <tt:Address><tt:Type>IPv4</tt:Type><tt:IPv4Address>0.0.0.0</tt:IPv4Address></tt:Address>
    <tt:Port>0</tt:Port>
    <tt:TTL>0</tt:TTL>
    <tt:AutoStart>false</tt:AutoStart>
  </tt:Multicast>
  <tt:SessionTimeout>PT10S</tt:SessionTimeout>
</trt:Configurations>
```

**MediaUri** (GetStreamUri and GetSnapshotUri response — all 4 fields required):
```xml
<trt:MediaUri>
  <tt:Uri>rtsp://192.168.1.100:554/stream</tt:Uri>
  <tt:InvalidAfterConnect>false</tt:InvalidAfterConnect>
  <tt:InvalidAfterReboot>false</tt:InvalidAfterReboot>
  <tt:Timeout>PT0S</tt:Timeout>
</trt:MediaUri>
```

### MediaService Trait (updated signatures)

```rust
#[async_trait]
pub trait MediaService: Send + Sync + 'static {
    /// Returns the RTSP streaming URI for the given profile token.
    async fn get_stream_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        not_implemented()
    }

    /// Returns the HTTP snapshot URI for the given profile token.
    async fn get_snapshot_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        not_implemented()
    }
}
```

`GetProfiles`, `GetVideoSources`, `GetVideoSourceConfigurations`, `GetVideoEncoderConfigurations` are all handler-internal (static from constants) — no trait delegation needed. This matches how `GetCapabilities` and `GetServices` are handler-internal in DeviceServiceHandler.

### extract_text_element helper (new private fn in media.rs)

```rust
fn extract_text_element(body: &Bytes, element_name: &str) -> Result<String, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    let mut inside_target = false;
    loop {
        match reader.read_resolved_event().map_err(|e| SoapFault::sender(format!("{e}")))? {
            (_, Event::Start(e)) => {
                let local = std::str::from_utf8(e.local_name().as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?;
                if local == element_name {
                    inside_target = true;
                }
            }
            (_, Event::Text(t)) if inside_target => {
                return t.unescape()
                    .map(|s| s.into_owned())
                    .map_err(|e| SoapFault::sender(format!("{e}")));
            }
            (_, Event::Eof) => return Err(SoapFault::sender(
                format!("Element {element_name} not found in body")
            )),
            _ => {}
        }
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|------------------|-------|
| Media2 (Profile T) with H.265 | Media (Profile S) with H.264/JPEG | Profile T is v2 scope — deferred in REQUIREMENTS.md |
| Single `SoapService` | `Router::merge()` of two `SoapService` routers | axum 0.8 merge works — confirmed by server.rs pattern |

---

## Open Questions

1. **Video encoder configuration token constant**
   - What we know: `VIDEO_SOURCE_TOKEN`, `PROFILE_TOKEN`, `PTZ_CONFIG_TOKEN` are defined. No `VIDEO_ENCODER_TOKEN` constant exists.
   - What's unclear: Should the video encoder token be a new constant in `constants.rs`?
   - Recommendation: Add `VIDEO_ENCODER_TOKEN: &str = "video_enc_0"` to `constants.rs`. Keep it consistent with the other token constants. The planner should create a task for this.

2. **Whether media_service should be optional at run() time**
   - What we know: Device service is required at `run()` time. Media service field exists but wiring is pending.
   - What's unclear: Should a consumer be able to start a server without a media service registered?
   - Recommendation: Require media_service at `run()` time with a clear error. Any ONVIF client will call GetProfiles immediately; returning errors for every media call is worse UX than a startup failure.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | tokio::test + std::test (no external test framework) |
| Config file | none — uses `[dev-dependencies]` in Cargo.toml |
| Quick run command | `cargo test -p onvif-server media` |
| Full suite command | `cargo test -p onvif-server` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| MEDIA-01 | GetProfiles returns profile with PTZConfiguration including DefaultContinuousPanTiltVelocitySpace | unit | `cargo test -p onvif-server media_get_profiles` | ❌ Wave 0 |
| MEDIA-01 | GetProfiles PTZConfiguration token = PTZ_CONFIG_TOKEN | unit | `cargo test -p onvif-server media_get_profiles_ptz_config_token` | ❌ Wave 0 |
| MEDIA-02 | GetStreamUri returns consumer-provided RTSP URL | unit | `cargo test -p onvif-server media_get_stream_uri` | ❌ Wave 0 |
| MEDIA-03 | GetVideoSources returns video source with VIDEO_SOURCE_TOKEN | unit | `cargo test -p onvif-server media_get_video_sources` | ❌ Wave 0 |
| MEDIA-04 | GetVideoSourceConfigurations returns config with VIDEO_SOURCE_TOKEN | unit | `cargo test -p onvif-server media_get_video_source_configurations` | ❌ Wave 0 |
| MEDIA-05 | GetVideoEncoderConfigurations returns config with encoding and resolution | unit | `cargo test -p onvif-server media_get_video_encoder_configurations` | ❌ Wave 0 |
| MEDIA-06 | GetSnapshotUri returns consumer-provided snapshot URL | unit | `cargo test -p onvif-server media_get_snapshot_uri` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p onvif-server`
- **Per wave merge:** `cargo test -p onvif-server`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `tests/media_service.rs` — covers MEDIA-01 through MEDIA-06
- [ ] `VIDEO_ENCODER_TOKEN` constant in `constants.rs` — referenced by handler and tests

---

## Sources

### Primary (HIGH confidence)

- `wsdl/media.wsdl` (bundled in repo) — GetProfiles, GetStreamUri, GetSnapshotUri, GetVideoSources, GetVideoSourceConfigurations, GetVideoEncoderConfigurations request/response element definitions
- `wsdl/onvif.xsd` (bundled in repo) — Profile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration, PTZConfiguration, MediaUri, VideoResolution, MulticastConfiguration type definitions
- `src/service/device.rs` (existing code) — DeviceServiceHandler pattern, extract_local_name implementation
- `src/server.rs` (existing code) — Router::merge pattern, ServerBuilder wiring, auth closure pattern
- `src/constants.rs` (existing code) — All token constants confirmed

### Secondary (MEDIUM confidence)

- ONVIF Profile S specification knowledge (training data, 2024 vintage) — namespace prefix `trt:` for media service confirmed by media.wsdl WSDL binding section
- Frigate PTZ autotracking behavior (training data) — DefaultContinuousPanTiltVelocitySpace requirement confirmed by CONTEXT.md and REQUIREMENTS.md

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already present in Cargo.toml
- Architecture: HIGH — confirmed from actual source files (device.rs, server.rs)
- XML type structures: HIGH — verified directly from bundled onvif.xsd and media.wsdl
- Pitfalls: HIGH — derived from direct XSD analysis (required/optional fields) and existing CONTEXT.md documentation of Frigate behavior

**Research date:** 2026-04-05
**Valid until:** 2026-05-05 (stable ONVIF spec, stable codebase)
