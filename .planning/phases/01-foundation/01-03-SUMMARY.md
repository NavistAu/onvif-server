---
phase: 01-foundation
plan: 03
subsystem: infra
tags: [rust, cargo, onvif, builder-pattern, auth-bypass, hashset, arc, thiserror]

# Dependency graph
requires:
  - phase: 01-foundation/01-01
    provides: OnvifError, module skeleton, crate scaffold
  - phase: 01-foundation/01-02
    provides: Five service traits (DeviceService, MediaService, PTZService, ImagingService, EventService), all Arc<dyn T>-safe

provides:
  - OnvifServerBuilder with port(), auth(), device_service(), media_service(), ptz_service(), imaging_service(), event_service() methods
  - OnvifServer struct storing all builder configuration fields (ready for Phase 2 server binding)
  - BuildError enum with MissingRequiredService variant
  - GetSystemDateAndTime auto-registered in auth_bypass set at builder construction (ONVIF spec requirement)
  - OnvifServerBuilder::auth_bypass_set() accessor for test and Phase 2 wiring access
  - Full Phase 1 test suite: all 6 foundation tests green, 0 ignored
  - BuildError re-exported from crate root

affects: [all Phase 2+ plans, soap_server::ServerBuilder wiring, auth wiring]

# Tech tracking
tech-stack:
  added: [std::collections::HashSet, thiserror (already present, now used for BuildError)]
  patterns:
    - OnvifServer::builder() factory method returns OnvifServerBuilder::new()
    - GetSystemDateAndTime inserted into auth_bypass in builder constructor — callers never manually register it
    - Service registration methods wrap impl Trait in Arc::new() and store as Option<Arc<dyn Trait>>
    - build() returns Ok(OnvifServer) skeleton — no port binding in Phase 1 (phase 2 concern)
    - auth_bypass_set() accessor pattern for exposing private HashSet fields to tests

key-files:
  created: []
  modified:
    - src/server.rs
    - src/lib.rs
    - tests/foundation.rs

key-decisions:
  - "build() returns Ok(OnvifServer) skeleton with no network activity — soap_server::ServerBuilder wiring deferred to Phase 2 per plan"
  - "GetSystemDateAndTime inserted into auth_bypass at OnvifServerBuilder::new() not build() — ensures invariant even if builder is cloned or inspected before build"
  - "auth_bypass field kept pub on OnvifServerBuilder plus auth_bypass_set() accessor added — pub on builder for Phase 2 wiring, accessor for clean test API"

patterns-established:
  - "Pattern 7: Builder factory via OnvifServer::builder() — Phase 2 extends build() to call soap_server::ServerBuilder without changing the public API"
  - "Pattern 8: GetSystemDateAndTime auto-bypass — Phase 2 must pass this set to soap_server::ServerBuilder::auth_bypass() without modification"

requirements-completed: [INFRA-06, INFRA-07, INFRA-08]

# Metrics
duration: 2min
completed: 2026-04-05
---

# Phase 1 Plan 03: OnvifServer Builder Skeleton Summary

**OnvifServerBuilder accepting all five service types via Arc<dyn Trait>, auth credentials, port config, and auto-registered GetSystemDateAndTime bypass — full Phase 1 test suite green (6 passed, 0 ignored)**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-04-05T07:31:32Z
- **Completed:** 2026-04-05T07:33:38Z
- **Tasks:** 2
- **Files modified:** 3 modified, 0 created

## Accomplishments

- `OnvifServer::builder().port(8080).auth("admin","secret").device_service(impl).build()` compiles and returns `Ok(OnvifServer)`
- `GetSystemDateAndTime` pre-inserted into `auth_bypass` HashSet at construction — callers never manually register it
- All five service traits accepted via `Arc<dyn T>` storage — no Arc visibility exposed to callers
- `BuildError` with `MissingRequiredService` variant re-exported from crate root
- Phase 1 complete: all 6 foundation tests pass with 0 ignored, `cargo build` clean

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: failing test bodies for builder** - `d8def4a` (test)
2. **Task 1 GREEN: OnvifServerBuilder implementation** - `12bac18` (feat)
3. **Task 2: enable all test stubs — full suite green** - `e88081d` (feat)

_Note: TDD flow — RED commit added test bodies (still #[ignore]), GREEN commit implemented builder, Task 2 commit removed #[ignore] attributes._

## Files Created/Modified

- `/Users/jhogendorn/ws/onvif-server/src/server.rs` - Full OnvifServerBuilder with port/auth/service methods, BuildError, OnvifServer struct, auth_bypass_set() accessor
- `/Users/jhogendorn/ws/onvif-server/src/lib.rs` - Added BuildError to re-exports
- `/Users/jhogendorn/ws/onvif-server/tests/foundation.rs` - Implemented test_builder_accepts_service_calls and test_auth_bypass_includes_get_system_date_and_time; removed all #[ignore] attributes

## Decisions Made

- **build() is a skeleton (no network activity):** Phase 2 will call `soap_server::ServerBuilder` inside `build()`. Phase 1's job is only to ensure the API surface compiles and stores fields correctly.
- **auth_bypass_set() accessor added:** The `auth_bypass` field is `pub` on `OnvifServerBuilder` for Phase 2 direct field access, and `auth_bypass_set()` provides a cleaner accessor for test assertions. Both exist for different callers.
- **GetSystemDateAndTime inserted in new() not build():** Ensures the invariant holds even if a caller inspects the builder before calling build(). Calling build() multiple times (not the intended usage) would still have the bypass present.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 1 is complete. All six foundation tests pass. `cargo build -p onvif-server` succeeds clean.
- Phase 2 (Device Management) can begin immediately.
- `OnvifServer` stores all configured fields; Phase 2 extends `build()` to call `soap_server::ServerBuilder::auth()` with `self.username`/`self.password` and `soap_server::ServerBuilder::auth_bypass()` with `self.auth_bypass`.
- Type strategy note from Plan 02 still applies: XSD codegen requires Rust 1.86+; crate pinned to 1.85.1. Phase 2 should continue with hand-written types or upgrade the toolchain pin first.

---
*Phase: 01-foundation*
*Completed: 2026-04-05*

## Self-Check: PASSED

- FOUND: src/server.rs
- FOUND: src/lib.rs
- FOUND: tests/foundation.rs
- FOUND: .planning/phases/01-foundation/01-03-SUMMARY.md
- FOUND: d8def4a (Task 1 RED commit)
- FOUND: 12bac18 (Task 1 GREEN commit)
- FOUND: e88081d (Task 2 commit)
