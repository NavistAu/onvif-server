---
phase: 03-media-service
verified: 2026-04-05T10:35:00Z
status: passed
score: 11/11 must-haves verified
re_verification: false
---

# Phase 03: Media Service Verification Report

**Phase Goal:** A consumer can configure stream URIs and snapshot URIs, and any ONVIF client can retrieve complete Profile S media metadata including profiles with correct PTZ configuration references
**Verified:** 2026-04-05T10:35:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

Must-haves sourced from 03-01-PLAN.md and 03-02-PLAN.md frontmatter `must_haves.truths`.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | GetProfiles returns a profile containing PTZConfiguration with DefaultContinuousPanTiltVelocitySpace set to TRANSLATION_SPACE_FOV | VERIFIED | `src/service/media.rs` lines 117-122: `<tt:PTZConfiguration token="{ptz_cfg_token}">` block includes `<tt:DefaultContinuousPanTiltVelocitySpace>{translation_space_fov}</tt:DefaultContinuousPanTiltVelocitySpace>` — variable bound to `TRANSLATION_SPACE_FOV` constant |
| 2 | GetStreamUri passes the ProfileToken from the request body to the MediaService trait and returns the consumer's RTSP URI | VERIFIED | `src/service/media.rs` lines 137-138: `extract_text_element(body, "ProfileToken")?` then `self.svc.get_stream_uri(&profile_token).await` — result inserted into response XML |
| 3 | GetSnapshotUri passes the ProfileToken from the request body to the MediaService trait and returns the consumer's snapshot URI | VERIFIED | `src/service/media.rs` lines 155-156: same pattern as GetStreamUri, delegates to `self.svc.get_snapshot_uri(&profile_token).await` |
| 4 | GetVideoSources returns a video source response using VIDEO_SOURCE_TOKEN | VERIFIED | `src/service/media.rs` lines 175-179: `<trt:VideoSources token="{vs_token}">` where `vs_token = VIDEO_SOURCE_TOKEN` |
| 5 | GetVideoSourceConfigurations returns a configuration referencing VIDEO_SOURCE_TOKEN | VERIFIED | `src/service/media.rs` lines 188-192: token attribute and `<tt:SourceToken>` both use `VIDEO_SOURCE_TOKEN` |
| 6 | GetVideoEncoderConfigurations returns a configuration with Encoding, Resolution, Multicast, and SessionTimeout elements | VERIFIED | `src/service/media.rs` lines 203-222: `<tt:Encoding>H264</tt:Encoding>`, `<tt:Resolution>`, `<tt:Multicast>` (with IPv4/Port/TTL/AutoStart children), and `<tt:SessionTimeout>PT10S</tt:SessionTimeout>` all present |
| 7 | OnvifServer::run() starts a server that responds to both /onvif/device_service and /onvif/media_service paths | VERIFIED | `src/server.rs` lines 68-103: two separate `ServerBuilder` blocks with `.path("/onvif/device_service")` and `.path("/onvif/media_service")`, merged via `soap_svc.into_router().merge(media_soap_svc.into_router())` |
| 8 | MediaServiceHandler is publicly exported from the crate root so consumers can reference it | VERIFIED | `src/lib.rs` line 21: `pub use service::media::MediaServiceHandler;` |
| 9 | Media type stubs (MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration) are exported from the crate root | VERIFIED | `src/lib.rs` lines 14-17: all 4 types in `pub use generated::{...}` block; `src/generated/mod.rs` re-exports them from `types` module |
| 10 | Auth credentials are cloned before building each SoapService — no move-after-use compile error | VERIFIED | `src/server.rs` lines 62-65: `username/password` cloned twice (`username2/password2`) before either closure captures them |
| 11 | media_service field is required at run() time with a clear error message | VERIFIED | `src/server.rs` line 50-51: `.ok_or("media_service is required to call run()")?` |

**Score:** 11/11 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/service/media.rs` | MediaServiceHandler with all 6 operation handlers | VERIFIED | 228 lines; implements `SoapHandler` with 6 match arms; `Arc<dyn MediaService>` field; all handlers substantive with real XML format strings |
| `src/traits/media.rs` | Typed MediaService trait with get_stream_uri(&str) and get_snapshot_uri(&str) | VERIFIED | 20 lines; trait has exactly 2 typed methods with `profile_token: &str` parameter and `Result<String, OnvifError>` return type; default implementations call `not_implemented()` |
| `tests/media_service.rs` | 7 active integration tests covering MEDIA-01 through MEDIA-06 | VERIFIED | 154 lines; 7 `#[tokio::test]` functions; zero `#[ignore]` attributes; all assertions are substantive (token values, element names, URI patterns) |
| `src/constants.rs` | VIDEO_ENCODER_TOKEN constant | VERIFIED | Line 23: `pub const VIDEO_ENCODER_TOKEN: &str = "video_enc_0";` |
| `src/server.rs` | run() wires MediaServiceHandler as second SoapService merged via Router::merge() | VERIFIED | Lines 85-103: full `media_soap_svc` ServerBuilder block with `media.wsdl`, `media_handler`, auth closure; merged at line 103 |
| `src/lib.rs` | pub use for MediaServiceHandler and new media types | VERIFIED | Lines 14-21: both `MediaServiceHandler` and all 4 media types exported |
| `src/generated/types.rs` | MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration type stubs | VERIFIED | Lines 41-78: all 4 structs present with `Debug + Clone` derives |
| `src/generated/mod.rs` | Re-exports 4 media types at module boundary | VERIFIED | Line 4: all 4 types in `pub use types::{...}` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/service/media.rs` | `src/traits/media.rs` | `Arc<dyn MediaService>` field on MediaServiceHandler | VERIFIED | Line 16: `pub(crate) svc: Arc<dyn MediaService>`; lines 138/156: `self.svc.get_stream_uri(...)` and `self.svc.get_snapshot_uri(...)` called |
| `src/service/media.rs` | `src/constants.rs` | PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV, VIDEO_ENCODER_TOKEN | VERIFIED | Lines 10-13: all 6 constants imported via `use crate::constants::{...}` and used in XML format strings |
| `src/server.rs` | `src/service/media.rs` | MediaServiceHandler construction in run() | VERIFIED | Line 6: `use crate::service::media::MediaServiceHandler;`; line 60: `MediaServiceHandler::new(media_svc, media_xaddr)` |
| `src/server.rs` | `soap_server::ServerBuilder` | Second ServerBuilder block for /onvif/media_service | VERIFIED | Lines 85-100: full `media_soap_svc` ServerBuilder with `media.wsdl`, path, handler, auth, auth_bypass |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| MEDIA-01 | 03-01, 03-02 | User can call GetProfiles and receive at least one media profile with video source, encoder, and PTZ configuration references | SATISFIED | `handle_get_profiles()` returns profile with VideoSourceConfiguration, VideoEncoderConfiguration, and PTZConfiguration elements; test `media_get_profiles` verifies PROFILE_TOKEN, PTZConfiguration element, DefaultContinuousPanTiltVelocitySpace, and TRANSLATION_SPACE_FOV URI |
| MEDIA-02 | 03-01, 03-02 | User can call GetStreamUri with a profile token and receive an RTSP URL as configured by the consumer | SATISFIED | `handle_get_stream_uri()` calls `extract_text_element(body, "ProfileToken")` then `self.svc.get_stream_uri(&profile_token)`; test `media_get_stream_uri` passes token "profile_0" and asserts "rtsp://test/profile_0" in response |
| MEDIA-03 | 03-01, 03-02 | User can call GetVideoSources and receive video source descriptions with resolution and frame rate | SATISFIED | `handle_get_video_sources()` returns VideoSources element with Framerate(30) and Resolution(1920x1080); test `media_get_video_sources` asserts VIDEO_SOURCE_TOKEN "video_src_0" present |
| MEDIA-04 | 03-01, 03-02 | User can call GetVideoSourceConfigurations and receive configurations linking video sources to profiles | SATISFIED | `handle_get_video_source_configurations()` returns Configurations with token, Name, UseCount, SourceToken (VIDEO_SOURCE_TOKEN), and Bounds; test `media_get_video_source_configurations` asserts "video_src_0" present |
| MEDIA-05 | 03-01, 03-02 | User can call GetVideoEncoderConfigurations and receive encoder settings (codec, resolution, bitrate) | SATISFIED | `handle_get_video_encoder_configurations()` returns Configurations with Encoding(H264), Resolution(1920x1080), RateControl (30fps/4096kbps), Multicast, SessionTimeout; test `media_get_video_encoder_configurations` asserts H264, Multicast, SessionTimeout |
| MEDIA-06 | 03-01, 03-02 | User can call GetSnapshotUri with a profile token and receive a snapshot URL as configured by the consumer | SATISFIED | `handle_get_snapshot_uri()` calls `extract_text_element(body, "ProfileToken")` then `self.svc.get_snapshot_uri(&profile_token)`; test `media_get_snapshot_uri` passes token "profile_0" and asserts "http://test/profile_0/snapshot.jpg" in response |

All 6 requirement IDs (MEDIA-01 through MEDIA-06) declared in both plan frontmatter blocks are satisfied. No orphaned requirements — REQUIREMENTS.md maps exactly MEDIA-01 through MEDIA-06 to Phase 3 and all are marked `[x]` complete.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | No anti-patterns found |

Scans performed:
- No `#[ignore]` attributes in `tests/media_service.rs`
- No `tds:` namespace (wrong namespace) in `src/service/media.rs`
- No TODO/FIXME/HACK/PLACEHOLDER comments in modified files
- No empty return values (`return null`, `return {}`) in handler implementations
- The `_ => {}` matches on lines 54 and 81 are legitimate XML parse loop catch-all arms, not stubs

---

### Human Verification Required

**1. Real ONVIF Client Interop Test**

**Test:** Point an actual ONVIF client (e.g., ONVIF Device Manager, Frigate's ONVIF autotracker, or iSpy) at a running server instance
**Expected:** Client successfully enumerates profiles, receives RTSP URI and snapshot URI, and parses PTZConfiguration including DefaultContinuousPanTiltVelocitySpace without errors
**Why human:** XML namespace correctness and SOAP envelope structure can only be validated end-to-end against a real ONVIF client parser; unit tests verify string content but not full SOAP envelope wrapping or schema conformance

**2. media.wsdl Presence**

**Test:** Confirm `wsdl/media.wsdl` exists and is a valid ONVIF Media Service WSDL (the file is `include_bytes!`'d at compile time so a build success is evidence, but content quality matters for WSDL-validation clients)
**Expected:** WSDL is the official ONVIF Profile S Media Service WSDL referencing correct namespaces
**Why human:** Cannot verify WSDL content correctness programmatically without running an ONVIF schema validator

---

### Gaps Summary

No gaps. All 11 must-have truths verified, all 8 artifacts substantive and wired, all 4 key links confirmed present, all 6 requirement IDs satisfied. The codebase matches the plan's stated intent.

---

_Verified: 2026-04-05T10:35:00Z_
_Verifier: Claude (gsd-verifier)_
