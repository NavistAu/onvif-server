---
phase: 01-foundation
plan: 01
subsystem: infra
tags: [rust, cargo, soap-server, thiserror, onvif, soap-fault]

# Dependency graph
requires: []
provides:
  - Compilable onvif-server crate with soap-server path dependency
  - OnvifError enum with ONVIF-namespaced SOAP fault mapping
  - Five crate-level pub const token strings (PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV)
  - Module skeleton: error, constants, server (stub), wsdl_loader (stub), traits (stub)
  - tests/foundation.rs with two passing tests and four ignored stubs
affects: [01-02, 01-03, all downstream plans]

# Tech tracking
tech-stack:
  added: [soap-server (path dep), tokio 1.x, async-trait 0.1, thiserror 2, bytes 1, chrono 0.4, uuid 1, quick-xml 0.39, rust-toolchain 1.85.1]
  patterns:
    - OnvifError.into_soap_fault() maps ONVIF errors to SoapFault with inline xmlns:ter detail
    - tokio sync-only in [dependencies], full only in [dev-dependencies]
    - Stub modules (server.rs, wsdl_loader.rs, traits/mod.rs) satisfy lib.rs module declarations

key-files:
  created:
    - Cargo.toml
    - rust-toolchain.toml
    - src/lib.rs
    - src/error.rs
    - src/constants.rs
    - src/server.rs
    - src/wsdl_loader.rs
    - src/traits/mod.rs
    - tests/foundation.rs
  modified: []

key-decisions:
  - "Pinned rust-toolchain.toml to 1.85.1 — cpufeatures 0.3.0 (transitive dep) requires edition2024, unsupported by system default 1.79.0"
  - "xmlns:ter namespace declared inline in SoapFault detail string — soap-server envelope does not inject ONVIF namespaces"
  - "tokio features=[sync] in lib deps only, features=[full,test-util] in dev-deps — prevents forcing full tokio on library consumers"

patterns-established:
  - "Pattern 1: OnvifError variants map to SoapFault via into_soap_fault() with ter: namespace in detail"
  - "Pattern 2: Stub modules with minimal pub types satisfy module declarations so crate compiles before implementation"
  - "Pattern 3: TDD — tests/foundation.rs stubs marked #[ignore] for future plans, active tests run immediately"

requirements-completed: [INFRA-01, INFRA-02, INFRA-09]

# Metrics
duration: 15min
completed: 2026-04-05
---

# Phase 1 Plan 01: Crate Scaffold Summary

**Compilable onvif-server crate with OnvifError SOAP fault mapping, five ONVIF token constants, and full module skeleton pinned to Rust 1.85.1**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-04-05T07:00:00Z
- **Completed:** 2026-04-05T07:15:00Z
- **Tasks:** 2
- **Files modified:** 9 created, 0 modified

## Accomplishments

- Crate compiles cleanly with `cargo build -p onvif-server` — single tokio 1.51.0 in dependency graph
- `OnvifError::NotImplemented.into_soap_fault()` detail contains `xmlns:ter="http://www.onvif.org/ver10/error"` (test passes)
- All five token constants exported and accessible as `onvif_server::PROFILE_TOKEN` etc. (test passes)
- Module skeleton in place for error, constants, server, wsdl_loader, and traits — ready for plans 02 and 03

## Task Commits

Each task was committed atomically:

1. **Task 1: Cargo.toml and crate root** - `acbeaf0` (feat)
2. **Task 2: OnvifError and token constants** - `4bfc16f` (feat)

_Note: TDD tasks had RED phase (test file with failing test_not_implemented_fault_has_ter_namespace) then GREEN phase (implemented into_soap_fault)._

## Files Created/Modified

- `/Users/jhogendorn/ws/onvif-server/Cargo.toml` - Crate manifest with soap-server path dep, correct tokio feature split
- `/Users/jhogendorn/ws/onvif-server/rust-toolchain.toml` - Pins to Rust 1.85.1 for edition2024 dep support
- `/Users/jhogendorn/ws/onvif-server/src/lib.rs` - Crate root declaring all modules and public re-exports
- `/Users/jhogendorn/ws/onvif-server/src/error.rs` - OnvifError enum with into_soap_fault() and not_implemented() helper
- `/Users/jhogendorn/ws/onvif-server/src/constants.rs` - Five pub const token strings with doc comments
- `/Users/jhogendorn/ws/onvif-server/src/server.rs` - Stub OnvifServer and OnvifServerBuilder structs
- `/Users/jhogendorn/ws/onvif-server/src/wsdl_loader.rs` - Stub EmbeddedWsdlLoader struct
- `/Users/jhogendorn/ws/onvif-server/src/traits/mod.rs` - Empty module placeholder
- `/Users/jhogendorn/ws/onvif-server/tests/foundation.rs` - Two passing tests, four ignored stubs

## Decisions Made

- **Rust 1.85.1 toolchain pin:** The system default Rust was 1.79.0. A transitive dependency (`cpufeatures 0.3.0`) requires `edition2024` which needs Rust 1.85+. Created `rust-toolchain.toml` pinned to 1.85.1 (Rule 3 auto-fix — blocking issue).
- **Inline xmlns:ter namespace:** The ONVIF spec requires `xmlns:ter` in the fault detail. soap-server's envelope wrapper does not inject ONVIF namespaces, so the declaration is embedded directly in the detail string format.
- **tokio feature split:** Library depends only on `sync` feature; `full` and `test-util` are dev-dependencies only. This prevents library consumers from inheriting an unnecessarily large tokio feature set.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added rust-toolchain.toml pinned to 1.85.1**
- **Found during:** Task 1 (first cargo build attempt)
- **Issue:** System Rust 1.79.0 cannot parse `edition2024` in `cpufeatures v0.3.0` Cargo.toml — `cargo build` failed immediately
- **Fix:** Created `rust-toolchain.toml` with `channel = "1.85.1"` (already installed via mise)
- **Files modified:** rust-toolchain.toml (new)
- **Verification:** `cargo build -p onvif-server` succeeded after toolchain switch
- **Committed in:** acbeaf0 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Essential infrastructure fix. No scope creep. The toolchain pin is required for any downstream plan to build.

## Issues Encountered

None beyond the toolchain version mismatch documented above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Crate compiles and all success criteria met
- Plans 02 (service traits) and 03 (server builder) can begin immediately
- `OnvifError` and all token constants are exported and ready for use in downstream plans
- Stub modules will be replaced with real implementations in plans 02 and 03

---
*Phase: 01-foundation*
*Completed: 2026-04-05*

## Self-Check: PASSED

- FOUND: Cargo.toml
- FOUND: src/lib.rs
- FOUND: src/error.rs
- FOUND: src/constants.rs
- FOUND: tests/foundation.rs
- FOUND: rust-toolchain.toml
- FOUND: .planning/phases/01-foundation/01-01-SUMMARY.md
- FOUND: acbeaf0 (Task 1 commit)
- FOUND: 4bfc16f (Task 2 commit)
