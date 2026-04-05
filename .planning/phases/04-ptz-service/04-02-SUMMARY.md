---
phase: 04-ptz-service
plan: 02
subsystem: api
tags: [onvif, ptz, soap, axum, async-trait, rust, frigate, integration-test]

requires:
  - phase: 04-ptz-service/04-01
    provides: PTZServiceHandler with all 15 operations, PTZService trait, PTZStatusResult/PTZPreset types
  - phase: 03-media-service
    provides: MediaServiceHandler pattern for server wiring (ServerBuilder block, Router::merge)
  - phase: 02-device-management
    provides: OnvifServer::run() with device_service and media_service slots, axum router pattern

provides:
  - PTZ service wired into OnvifServer::run() at /onvif/ptz_service via Router::merge() (third block)
  - frigate_autotracker_call_sequence integration test validating complete 7-step Frigate startup sequence
  - virtual_ptz example with Clone-able VirtualPTZ implementing all three service traits via Arc<Mutex<HashMap>>

affects: [05-integration-test, frigate-compat, consumer-docs]

tech-stack:
  added: []
  patterns:
    - "OnvifServer::run() requires three services: device_service, media_service, ptz_service — all extracted via ok_or()"
    - "Three username/password clone pairs before closures: username/password, username2/password2, username3/password3"
    - "examples/ uses Clone-derived struct with Arc<Mutex<_>> state for multi-slot service registration"
    - "Integration tests call handlers directly via SoapHandler::handle() — no HTTP server needed"

key-files:
  created:
    - tests/frigate_compat.rs
    - examples/virtual_ptz.rs
  modified:
    - src/server.rs
    - tests/device_management.rs

key-decisions:
  - "ptz_service required at run() time via ok_or() — same pattern as device_service and media_service"
  - "VirtualPTZ derives Clone and uses Arc<Mutex<HashMap>> internally — single instance cloned for all three service slots"
  - "frigate_compat.rs uses dedicated TestMediaFrigate/TestPTZFrigate stubs rather than reusing other test stubs"

patterns-established:
  - "Pattern: Three service username/password clone pairs at top of run() before any closures"
  - "Pattern: Example structs use Arc<Mutex<_>> + #[derive(Clone)] for shared-state multi-service registration"
  - "Pattern: Frigate compat integration test calls handlers directly via SoapHandler::handle(body).await"

requirements-completed:
  - TEST-01
  - TEST-02

duration: 4min
completed: 2026-04-05
---

# Phase 4 Plan 2: PTZ Service Wiring and Frigate Compat Test Summary

**PTZServiceHandler wired into OnvifServer::run() at /onvif/ptz_service, 7-step Frigate autotracker call sequence test green, and virtual_ptz example demonstrating complete three-service consumer API**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-04-05T11:32:16Z
- **Completed:** 2026-04-05T11:37:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- PTZ service fully wired into OnvifServer::run() as the third Router::merge() block at /onvif/ptz_service, following the same ServerBuilder pattern as device and media services
- frigate_autotracker_call_sequence test validates the complete Frigate PTZ autotracker startup sequence (GetProfiles -> GetConfigurationOptions -> GetServiceCapabilities -> GetPresets -> GetStatus -> RelativeMove -> GotoPreset) with exact Frigate-critical XML assertions (TranslationSpaceFov URI, MoveStatus="true" attribute, IDLE/PanTilt/Zoom in GetStatus)
- virtual_ptz example compiles and demonstrates Clone-able VirtualPTZ implementing all three service traits with in-memory preset storage shared across service slots via Arc<Mutex<HashMap>>
- Full test suite 32 active tests (8 device + 6 foundation + 1 frigate + 7 media + 10 ptz) all green

## Task Commits

1. **Task 1: Wire PTZServiceHandler into OnvifServer::run() at /onvif/ptz_service** - `a1d5463` (feat)
2. **Task 2: Frigate autotracker compat integration test and virtual_ptz example** - `bb7877c` (feat)

## Files Created/Modified

- `src/server.rs` - Added PTZServiceHandler import, ptz_svc extraction, third username3/password3 clone pair, PTZ ServerBuilder block at /onvif/ptz_service, Router::merge() for three services
- `tests/frigate_compat.rs` - frigate_autotracker_call_sequence test with TestMediaFrigate/TestPTZFrigate stubs and 7-step Frigate startup call sequence validation
- `examples/virtual_ptz.rs` - VirtualPTZ with Arc<Mutex<HashMap>> preset storage, Clone-derived, implements DeviceService + MediaService + PTZService; main() starts server on :8080 with admin/admin
- `tests/device_management.rs` - Added TestPTZ stub and .ptz_service(TestPTZ) to device_server_binds_and_serves_auth_exempt_op (auto-fix for regression caused by run() now requiring ptz_service)

## Decisions Made

- ptz_service required at run() time via ok_or() — consistent with device_service and media_service; callers must register all three services before calling run()
- VirtualPTZ derives Clone and uses Arc<Mutex<_>> internally so the same logical instance can be registered in all three service slots without duplicating state
- frigate_compat.rs has its own dedicated stubs (TestMediaFrigate, TestPTZFrigate) rather than importing from other test files — clearer test ownership

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] device_server_binds_and_serves_auth_exempt_op broke after run() required ptz_service**
- **Found during:** Task 2 verification (cargo test -p onvif-server)
- **Issue:** The existing device_management.rs integration test builds an OnvifServer with only device_service + media_service and calls run(). After Task 1 added ptz_service as a required service in run(), this test panicked with "ptz_service is required to call run()"
- **Fix:** Added TestPTZ stub (empty PTZService impl using all defaults) to device_management.rs and added .ptz_service(TestPTZ) to the server builder in that test
- **Files modified:** tests/device_management.rs
- **Verification:** cargo test -p onvif-server passes; all 32 active tests green
- **Committed in:** bb7877c (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 regression bug)
**Impact on plan:** Fix required for test suite to stay green. No scope changes.

## Issues Encountered

None beyond the auto-fixed regression above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All three ONVIF services (Device, Media, PTZ) fully wired into OnvifServer::run()
- Frigate autotracker compatibility validated end-to-end
- virtual_ptz example ready for use as consumer documentation
- Phase 5 (integration/ODM smoke test) can now use a fully functional three-service server
- TEST-01 (frigate_compat.rs) and TEST-02 (virtual_ptz example) requirements satisfied

---
*Phase: 04-ptz-service*
*Completed: 2026-04-05*

## Self-Check: PASSED

- FOUND: src/server.rs
- FOUND: tests/frigate_compat.rs
- FOUND: examples/virtual_ptz.rs
- FOUND: .planning/phases/04-ptz-service/04-02-SUMMARY.md
- FOUND: commit a1d5463 (Task 1)
- FOUND: commit bb7877c (Task 2)
