---
phase: 03-media-service
plan: 02
subsystem: api
tags: [onvif, soap, axum, media-service, router-merge, rust]

# Dependency graph
requires:
  - phase: 03-media-service
    plan: 01
    provides: MediaServiceHandler with 6 ONVIF operations + 4 media type stubs
  - phase: 02-device-management
    plan: 02
    provides: OnvifServer::run() with single-service device router
provides:
  - OnvifServer::run() wiring both Device and Media services via axum Router::merge()
  - media_service required at run() time with descriptive error
  - MediaServiceHandler exported from crate root as onvif_server::MediaServiceHandler
  - MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration exported from crate root
affects:
  - 04-ptz-service (PTZ handler will follow same Router::merge() wiring pattern)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dual ServerBuilder blocks: one for devicemgmt.wsdl, one for media.wsdl — each with own auth closure"
    - "Credential clone pattern: username/password cloned twice before closures consume first clone"
    - "Router::merge() via .merge() method chain on axum Router from soap_svc.into_router()"
    - "auth_bypass passed to device service; std::iter::empty::<String>() for media service"

key-files:
  created: []
  modified:
    - src/server.rs
    - src/lib.rs
    - src/generated/mod.rs
    - tests/device_management.rs

key-decisions:
  - "media_service is required at run() time — enforced via ok_or() just like device_service"
  - "std::iter::empty::<String>() for media auth_bypass — GetSystemDateAndTime is device-service-only"
  - "generated/mod.rs must re-export new types before lib.rs can re-export them from the crate root (missing in plan interface spec)"

# Metrics
duration: 5min
completed: 2026-04-05
---

# Phase 03 Plan 02: Server Wiring Summary

**Dual-service axum router with Device and Media services merged via Router::merge(), both exported from crate root, full test suite green (21 passed)**

## Performance

- **Duration:** 5 min
- **Started:** 2026-04-05T10:23:27Z
- **Completed:** 2026-04-05T10:28:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- OnvifServer::run() now wires two SoapService instances — one for devicemgmt.wsdl at /onvif/device_service and one for media.wsdl at /onvif/media_service
- Router::merge() combines both routers into a single axum router served on one port
- media_service required at run() time via ok_or() with clear error message
- Auth credentials cloned twice so each service's auth closure captures its own copy — no move-after-use errors
- MediaServiceHandler and 4 media types (MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration) exported from crate root
- Full test suite: 21 passed, 2 ignored, 0 failed

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire MediaServiceHandler in run() with Router::merge()** — `966c8f0` (feat)
2. **Task 2: Export MediaServiceHandler and media types from crate root** — `593b3f2` (feat)

## Files Created/Modified

- `src/server.rs` — Added MediaServiceHandler import, media_service extraction, dual credential clones, media ServerBuilder block, Router::merge()
- `src/lib.rs` — Extended pub use generated block with 4 media types
- `src/generated/mod.rs` — Added 4 media types to re-export list (required before lib.rs can re-export them)
- `tests/device_management.rs` — Added TestMedia stub and registered media_service in integration test server builder

## Decisions Made

- media_service is required at run() time — same enforcement pattern as device_service, consistent API
- std::iter::empty::<String>() for media service auth_bypass — GetSystemDateAndTime bypass is device-service-only and auth_bypass is already moved into the device service auth_bypass.into_iter()
- generated/mod.rs must re-export types before lib.rs can pick them up; the plan interface spec only showed the lib.rs change but not the intermediate generated/mod.rs step

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added media types to generated/mod.rs before lib.rs export**
- **Found during:** Task 2
- **Issue:** lib.rs `pub use generated::{..., MediaProfile, ...}` failed to compile — the `generated` module only re-exported device types in its mod.rs; the 4 new media types existed in `generated::types` but weren't surfaced at the `generated` module level
- **Fix:** Added `MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration` to the `pub use types::{...}` block in `src/generated/mod.rs`
- **Files modified:** src/generated/mod.rs
- **Verification:** cargo build succeeds
- **Committed in:** 593b3f2 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed device_management integration test missing media_service registration**
- **Found during:** Task 2 (cargo test)
- **Issue:** `device_server_binds_and_serves_auth_exempt_op` test built OnvifServer with only device_service registered; run() now requires media_service too — test panicked with "media_service is required to call run()"
- **Fix:** Added `TestMedia` no-op struct implementing `MediaService` (both methods use default not_implemented()); registered `.media_service(TestMedia)` in the server builder in the test
- **Files modified:** tests/device_management.rs
- **Verification:** cargo test — all 21 tests pass
- **Committed in:** 593b3f2 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking, 1 Rule 1 bug)
**Impact on plan:** Both fixes were minor and necessary. The generated/mod.rs gap was an omission in the plan interface spec. The test fix was a direct consequence of the new run() requirement added in Task 1.

## Issues Encountered

None beyond the auto-fixed deviations above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 3 (Media Service) is complete — OnvifServer serves both Device and Media endpoints
- Phase 4 (PTZ Service) can follow the same dual-ServerBuilder + Router::merge() wiring pattern established here
- No blockers

## Self-Check: PASSED

- FOUND: src/server.rs (Router::merge present at line 103)
- FOUND: src/lib.rs (MediaServiceHandler export, 4 media type exports)
- FOUND: src/generated/mod.rs (4 media types re-exported)
- FOUND: tests/device_management.rs (TestMedia stub, .media_service(TestMedia) registered)
- FOUND: commit 966c8f0 (Task 1)
- FOUND: commit 593b3f2 (Task 2)
- cargo test: 21 passed, 2 ignored, 0 failed

---
*Phase: 03-media-service*
*Completed: 2026-04-05*
