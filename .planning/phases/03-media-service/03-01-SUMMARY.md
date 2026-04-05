---
phase: 03-media-service
plan: 01
subsystem: api
tags: [onvif, soap, xml, media-service, profile-s, quick-xml, rust]

# Dependency graph
requires:
  - phase: 02-device-management
    provides: DeviceServiceHandler pattern (SoapHandler trait, extract_local_name, format-string XML responses)
provides:
  - MediaServiceHandler implementing SoapHandler with 6 ONVIF Profile S operations
  - Typed MediaService trait with get_stream_uri(&str) and get_snapshot_uri(&str)
  - VIDEO_ENCODER_TOKEN constant in constants.rs
  - MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration type stubs
  - 7 passing integration tests covering MEDIA-01 through MEDIA-06
affects:
  - 03-media-service/03-02 (wiring MediaServiceHandler into run())
  - 04-ptz-service (PTZ handler will mirror MediaServiceHandler pattern)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "MediaServiceHandler mirrors DeviceServiceHandler: struct + SoapHandler impl + private fn dispatch"
    - "extract_text_element: private fn to walk XML body and return text content of named element"
    - "Static XML responses via format! strings with trt: namespace prefix for Media service"
    - "BytesText text content read via std::str::from_utf8(t.as_ref()) in quick-xml 0.39"

key-files:
  created:
    - src/service/media.rs
    - tests/media_service.rs
  modified:
    - src/constants.rs
    - src/generated/types.rs
    - src/traits/media.rs
    - src/service/mod.rs
    - src/lib.rs

key-decisions:
  - "MediaService trait has only 2 methods (get_stream_uri, get_snapshot_uri); all other operations are handler-internal static responses from constants"
  - "extract_text_element uses t.as_ref() not t.unescape() for BytesText in quick-xml 0.39"
  - "GetProfiles PTZConfiguration includes DefaultContinuousPanTiltVelocitySpace=TRANSLATION_SPACE_FOV for Frigate compatibility"
  - "VideoEncoderConfiguration includes zeroed Multicast and PT10S SessionTimeout as XSD-required fields"

patterns-established:
  - "Handler-internal vs trait-delegated: static operations build from constants; only consumer-specific URI ops hit the trait"
  - "trt: namespace prefix for Media service (vs tds: for Device service)"

requirements-completed: [MEDIA-01, MEDIA-02, MEDIA-03, MEDIA-04, MEDIA-05, MEDIA-06]

# Metrics
duration: 5min
completed: 2026-04-05
---

# Phase 03 Plan 01: Media Service Handler Summary

**MediaServiceHandler with 6 ONVIF Profile S operations, typed trait, 4 type stubs, VIDEO_ENCODER_TOKEN constant, and 7 green integration tests**

## Performance

- **Duration:** 5 min
- **Started:** 2026-04-05T10:14:35Z
- **Completed:** 2026-04-05T10:19:52Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- Full MediaServiceHandler dispatching all 6 operations: GetProfiles, GetStreamUri, GetSnapshotUri, GetVideoSources, GetVideoSourceConfigurations, GetVideoEncoderConfigurations
- GetProfiles XML includes PTZConfiguration with DefaultContinuousPanTiltVelocitySpace set to TRANSLATION_SPACE_FOV — the critical Frigate PTZ autotracking field
- GetStreamUri and GetSnapshotUri parse ProfileToken from body and delegate to typed MediaService trait
- All 7 MEDIA integration tests pass; all existing device and foundation tests remain green

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold — constants, types, trait, test stubs** - `66fe321` (feat)
2. **Task 2: Implement MediaServiceHandler with all 6 operations** - `f71b955` (feat)
3. **Task 3: Enable tests and verify all media tests pass** - `9c2a142` (test)

## Files Created/Modified

- `src/constants.rs` — Added VIDEO_ENCODER_TOKEN = "video_enc_0"
- `src/generated/types.rs` — Added MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration stubs
- `src/traits/media.rs` — Replaced placeholder trait with typed get_stream_uri(&str)/get_snapshot_uri(&str) signatures
- `src/service/mod.rs` — Added `pub mod media;`
- `src/service/media.rs` — New: MediaServiceHandler with 6 operation handlers + extract_local_name + extract_text_element
- `src/lib.rs` — Added `pub use service::media::MediaServiceHandler;`
- `tests/media_service.rs` — New: 7 integration tests covering MEDIA-01 through MEDIA-06

## Decisions Made

- MediaService trait has only 2 methods (get_stream_uri, get_snapshot_uri). GetProfiles, GetVideoSources, GetVideoSourceConfigurations, GetVideoEncoderConfigurations are handler-internal and build static XML from constants — same pattern as GetCapabilities/GetServices in DeviceServiceHandler.
- quick-xml 0.39 `BytesText` does not have `.unescape()` — text content is read via `std::str::from_utf8(t.as_ref())`. The research code example used `.unescape()` which does not compile; fixed inline.
- `e.local_name()` in quick-xml 0.39 returns a temporary that must be bound to a variable before calling `.as_ref()` on it to avoid lifetime errors.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed BytesText::unescape() API mismatch in extract_text_element**
- **Found during:** Task 2 (implement MediaServiceHandler)
- **Issue:** Research code example used `t.unescape().map(|s| s.into_owned())` on `BytesText` but quick-xml 0.39 does not expose `unescape()` on `BytesText` — it is only available on `BytesCData`
- **Fix:** Replaced with `std::str::from_utf8(t.as_ref()).map(|s| s.to_owned())` — consistent with how soap-server/src/envelope.rs reads text bytes
- **Files modified:** src/service/media.rs
- **Verification:** cargo build succeeds
- **Committed in:** f71b955 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed lifetime error for e.local_name() temporary in extract_text_element**
- **Found during:** Task 2 (implement MediaServiceHandler)
- **Issue:** `std::str::from_utf8(e.local_name().as_ref())` dropped `e.local_name()` temporary before the borrow was used in `if local == element_name`
- **Fix:** Bound `e.local_name()` to a local variable `local_name` to extend the lifetime
- **Files modified:** src/service/media.rs
- **Verification:** cargo build succeeds with no errors
- **Committed in:** f71b955 (Task 2 commit)

**3. [Rule 2 - Missing Critical] Created stub MediaServiceHandler in Task 1 to allow test file to compile**
- **Found during:** Task 1 (scaffold)
- **Issue:** tests/media_service.rs imports MediaServiceHandler which doesn't exist yet — but Task 1's done criteria requires `cargo test passes (all existing tests green; new media tests compile but are ignored)`
- **Fix:** Created a minimal stub src/service/media.rs with a non-functional SoapHandler impl so the test file compiles
- **Files modified:** src/service/media.rs (stub), src/service/mod.rs, src/lib.rs
- **Verification:** cargo test passes with 7 ignored tests
- **Committed in:** 66fe321 (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 1 bugs, 1 Rule 2 missing critical)
**Impact on plan:** All auto-fixes necessary for compilation. Research code examples had API mismatches with quick-xml 0.39; fixes were minimal and correct.

## Issues Encountered

None beyond the auto-fixed deviations above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- MediaServiceHandler is complete and fully tested — ready for Plan 02 to wire it into `OnvifServer::run()`
- Plan 02 needs to: add MediaServiceHandler to server.rs run(), merge router with device router, add media.wsdl ServerBuilder block
- No blockers

---
*Phase: 03-media-service*
*Completed: 2026-04-05*
