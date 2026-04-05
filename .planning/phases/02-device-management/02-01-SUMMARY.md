---
phase: 02-device-management
plan: "01"
subsystem: api
tags: [rust, onvif, soap, quick-xml, chrono, async-trait]

requires:
  - phase: 01-foundation
    provides: "OnvifServer skeleton, SoapHandler trait, SoapFault types, OnvifError, DeviceInfo stub, EmbeddedWsdlLoader"

provides:
  - "DeviceServiceHandler in src/service/device.rs implementing SoapHandler"
  - "Scope, ScopeDefinition, HostnameInformation, NetworkInterface types in src/generated/types.rs"
  - "DeviceService trait with typed return signatures for all 7 operations"
  - "extract_local_name() for parsing SOAP operation names from raw bytes"
  - "Integration test stubs for all DEV-01 through DEV-07 + auth (9 total)"
  - "4 passing tests: GetSystemDateAndTime, GetCapabilities, GetServices, GetDeviceInformation"

affects:
  - "02-02: adds GetScopes, GetHostname, GetNetworkInterfaces and wires server.run()"
  - "03-media: same Arc<dyn Service> + SoapHandler pattern to follow"

tech-stack:
  added: []
  patterns:
    - "Arc<dyn DeviceService> field on handler — consumer supplies impl at construction time"
    - "SoapHandler dispatches via extract_local_name() match on operation name"
    - "Handler returns inner body XML only — soap-server adds envelope wrapper"
    - "DeviceService trait defaults: sensible (get_system_date_and_time → Utc::now()) or not_implemented()"
    - "Integration tests call handler.handle() directly with minimal SOAP body bytes"

key-files:
  created:
    - src/service/mod.rs
    - src/service/device.rs
    - tests/device_management.rs
  modified:
    - src/generated/types.rs
    - src/generated/mod.rs
    - src/traits/device.rs
    - src/error.rs
    - src/lib.rs
    - tests/foundation.rs
    - Cargo.toml

key-decisions:
  - "GetCapabilities and GetServices are handler-internal — not on DeviceService trait; handler builds XML from bound xaddr"
  - "not_implemented() made generic (Result<T, OnvifError>) to serve typed trait stubs"
  - "get_system_date_and_time defaults to Ok(Utc::now()) — not an error — aligns with ONVIF spec requirement that device always reports time"
  - "SoapHandler imported from soap-server dev-dependency in integration tests"

patterns-established:
  - "Service handler pattern: struct { svc: Arc<dyn Trait>, xaddr: String } + impl SoapHandler"
  - "Operation dispatch: extract_local_name() → match → handle_* method"
  - "TDD style: call handler.handle(minimal_body_bytes).await in integration tests"

requirements-completed: [DEV-01, DEV-02, DEV-03, DEV-04]

duration: 20min
completed: "2026-04-05"
---

# Phase 2 Plan 01: Device Management Handler Foundation Summary

**DeviceServiceHandler dispatching GetSystemDateAndTime, GetCapabilities, GetServices, GetDeviceInformation via quick_xml operation parsing with typed DeviceService trait**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-04-05T07:45:00Z
- **Completed:** 2026-04-05T08:02:38Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- DeviceServiceHandler built and wired: `Arc<dyn DeviceService>` + xaddr, implements `SoapHandler`, dispatches 7 operations
- All 4 core ONVIF operations return spec-compliant XML with correct namespace declarations (tds: + tt:)
- DeviceService trait updated with typed returns for all 7 operations (removed get_ntp, get_users, get_system_uris)
- 4 integration tests pass calling the handler directly; 5 stubs remain #[ignore] for Plan 02

## Task Commits

1. **Task 1: Expand types, update trait signatures, add test stubs** - `a8dfbba` (feat)
2. **Task 2: DeviceServiceHandler with 4 core operations** - `25f97bd` (feat)

## Files Created/Modified

- `src/service/mod.rs` - Module declaration for service layer
- `src/service/device.rs` - DeviceServiceHandler: SoapHandler impl, extract_local_name(), 7 operation handlers
- `src/generated/types.rs` - Added Scope, ScopeDefinition, HostnameInformation, NetworkInterface
- `src/generated/mod.rs` - Re-exports new types
- `src/traits/device.rs` - Updated all 7 method signatures to typed returns; removed 3 non-required ops
- `src/error.rs` - Made not_implemented() generic: Result<T, OnvifError>
- `src/lib.rs` - Added service module, DeviceServiceHandler re-export, new type re-exports
- `tests/device_management.rs` - 9 test stubs (4 active, 5 #[ignore])
- `tests/foundation.rs` - Updated test to match new get_system_date_and_time default behavior
- `Cargo.toml` - Added soap-server dev-dependency

## Decisions Made

- GetCapabilities and GetServices are framework-level operations — the handler constructs their XML from the bound xaddr. They are NOT on the DeviceService trait.
- `not_implemented()` made generic so typed stubs (`Result<DeviceInfo, OnvifError>`) can use it as a one-liner.
- `get_system_date_and_time` defaults to `Ok(chrono::Utc::now())` — always returns time without requiring implementor override, matching ONVIF spec intent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Made not_implemented() generic**
- **Found during:** Task 1 (trait signature update)
- **Issue:** not_implemented() returned Result<(), OnvifError> — incompatible with new typed trait signatures like Result<DeviceInfo, OnvifError>
- **Fix:** Changed signature to `pub fn not_implemented<T>() -> Result<T, OnvifError>`
- **Files modified:** src/error.rs
- **Verification:** cargo build succeeds, existing tests pass
- **Committed in:** a8dfbba (Task 1 commit)

**2. [Rule 1 - Bug] Updated foundation test for new default behavior**
- **Found during:** Task 1 verification
- **Issue:** test_not_implemented_returns_error tested get_system_date_and_time returning Err, but the new default returns Ok(Utc::now())
- **Fix:** Changed test to assert get_device_information returns Err(NotImplemented) and get_system_date_and_time returns Ok
- **Files modified:** tests/foundation.rs
- **Verification:** All 6 foundation tests pass
- **Committed in:** a8dfbba (Task 1 commit)

**3. [Rule 3 - Blocking] Added soap-server dev-dependency**
- **Found during:** Task 2 test implementation
- **Issue:** Integration tests needed to import SoapHandler trait from soap-server; not listed as dev-dependency
- **Fix:** Added `soap-server = { path = "../soap-server" }` to [dev-dependencies] in Cargo.toml
- **Files modified:** Cargo.toml
- **Verification:** Tests compile and pass
- **Committed in:** 25f97bd (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 1 bugs, 1 Rule 3 blocker)
**Impact on plan:** All auto-fixes required for correctness and compilation. No scope creep.

## Issues Encountered

None beyond the auto-fixed deviations above.

## Next Phase Readiness

- Plan 02-02 can immediately add GetScopes, GetHostname, GetNetworkInterfaces by implementing the 3 stub methods in DeviceServiceHandler and removing #[ignore] from the corresponding tests
- The Arc<dyn DeviceService> + SoapHandler pattern is established and ready for server wiring in 02-02

---
*Phase: 02-device-management*
*Completed: 2026-04-05*

## Self-Check: PASSED

- FOUND: src/service/device.rs
- FOUND: src/service/mod.rs
- FOUND: tests/device_management.rs
- FOUND: .planning/phases/02-device-management/02-01-SUMMARY.md
- Commit a8dfbba: FOUND
- Commit 25f97bd: FOUND
