---
phase: 04-ptz-service
plan: 01
subsystem: api
tags: [onvif, ptz, soap, quick-xml, async-trait, rust]

requires:
  - phase: 03-media-service
    provides: MediaServiceHandler pattern (extract_local_name dispatch, format-string XML, Arc<dyn Trait>)
  - phase: 02-device-management
    provides: OnvifError enum, constants (PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV)

provides:
  - PTZServiceHandler implementing SoapHandler for all 15 ONVIF PTZ operations
  - PTZService trait with 9 typed control method signatures
  - PTZStatusResult and PTZPreset types in generated/types.rs
  - extract_element_attribute helper for XML attribute parsing (PanTilt x/y)
  - 10 active PTZ unit tests covering all Frigate compatibility requirements

affects: [05-server-wiring, frigate-compat-test, virtual-ptz-example]

tech-stack:
  added: []
  patterns:
    - "PTZServiceHandler mirrors MediaServiceHandler: extract_local_name + match dispatch"
    - "Discovery ops (GetNodes, GetConfigurations, etc.) are handler-internal static XML — not on trait"
    - "Control ops delegate to Arc<dyn PTZService> with typed parameters"
    - "extract_element_attribute helper handles XML attribute parsing (vs text element parsing)"
    - "Temporary value lifetimes in quick-xml: bind local_name to a let before borrowing as str"

key-files:
  created:
    - src/service/ptz.rs
    - tests/ptz_service.rs
  modified:
    - src/generated/types.rs
    - src/generated/mod.rs
    - src/traits/ptz.rs
    - src/service/mod.rs
    - src/lib.rs

key-decisions:
  - "PTZService trait has 9 control methods only; 6 discovery operations are handler-internal static XML"
  - "OnvifError has no NotFound variant — used InvalidArgument for unknown token errors in GetNode/GetConfiguration"
  - "extract_element_attribute returns Ok(None) when element absent — callers default to 0.0 for f32 params"
  - "PTZServiceHandler has no xaddr field — PTZ service does not advertise a separate service URL"
  - "Temporary quick-xml value lifetime fix: bind e.local_name() to a let binding before str conversion"

patterns-established:
  - "Pattern: extract_element_attribute(body, element_name, attr_name) for XML attribute extraction"
  - "Pattern: GetStatus serializes PTZStatusResult to nested <tt:MoveStatus><tt:PanTilt>IDLE/MOVING"
  - "Pattern: GetServiceCapabilities returns MoveStatus as XML attribute on Capabilities element"
  - "Pattern: Stop defaults pan_tilt/zoom to true when elements absent from request body"

requirements-completed:
  - PTZ-01
  - PTZ-02
  - PTZ-03
  - PTZ-04
  - PTZ-05
  - PTZ-06
  - PTZ-07
  - PTZ-08
  - PTZ-09
  - PTZ-10
  - PTZ-11
  - PTZ-12
  - PTZ-13
  - PTZ-14
  - PTZ-15

duration: 5min
completed: 2026-04-05
---

# Phase 4 Plan 1: PTZ Service Handler Summary

**PTZServiceHandler with all 15 ONVIF PTZ operations, typed PTZService trait, Frigate autotracker compatibility baked in via exact TranslationSpaceFov URI, MoveStatus-as-attribute, and nested GetStatus response**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-04-05T11:25:31Z
- **Completed:** 2026-04-05T11:29:40Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- PTZServiceHandler dispatches all 15 operations (6 discovery static XML + 9 control trait-delegated) following the proven MediaServiceHandler pattern
- All four Frigate autotracker pitfalls addressed: exact TRANSLATION_SPACE_FOV URI in GetNodes and GetConfigurationOptions, MoveStatus as XML attribute in GetServiceCapabilities, nested PanTilt/Zoom elements in GetStatus, PanTilt x/y as XML attributes in RelativeMove
- 10 active PTZ unit tests all green; full test suite 33 tests pass with no regressions

## Task Commits

1. **Task 1: Add PTZ types and rewrite PTZService trait** - `d479e46` (feat)
2. **Task 2: Implement PTZServiceHandler with all 15 operations** - `f12984e` (feat)
3. **Task 3: Wave-0 test scaffold and lib.rs exports** - `66062c2` (test)

## Files Created/Modified

- `src/service/ptz.rs` - PTZServiceHandler with 15 operation handlers and extract_element_attribute helper
- `tests/ptz_service.rs` - 10 active PTZ tests covering all Frigate compatibility requirements
- `src/generated/types.rs` - Added PTZStatusResult and PTZPreset structs
- `src/generated/mod.rs` - Re-export PTZStatusResult and PTZPreset
- `src/traits/ptz.rs` - Rewrote from stub to 9 typed method signatures
- `src/service/mod.rs` - Added `pub mod ptz`
- `src/lib.rs` - Exported PTZServiceHandler, PTZStatusResult, PTZPreset

## Decisions Made

- PTZService trait has 9 control methods only; discovery operations (GetNodes, GetConfigurations, GetConfiguration, GetConfigurationOptions, GetNode, GetServiceCapabilities) are handler-internal and return static XML
- OnvifError has no NotFound variant — used InvalidArgument for unknown token errors in GetNode and GetConfiguration token-mismatch paths
- extract_element_attribute returns Ok(None) when element is absent, callers default to 0.0 for f32 parameters
- PTZServiceHandler has no xaddr field — PTZ service does not advertise a separate service URL (unlike MediaServiceHandler)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] OnvifError::NotFound does not exist**
- **Found during:** Task 2 (PTZServiceHandler implementation)
- **Issue:** Plan specified `OnvifError::NotFound.into_soap_fault()` for unknown token errors, but the enum only has NotImplemented, InvalidArgument, ActionNotSupported variants
- **Fix:** Used `OnvifError::InvalidArgument(format!("Unknown NodeToken: {token}"))` and same for ConfigurationToken
- **Files modified:** src/service/ptz.rs
- **Verification:** cargo build passes
- **Committed in:** f12984e (Task 2 commit)

**2. [Rule 1 - Bug] quick-xml temporary value lifetime error in extract_element_attribute**
- **Found during:** Task 2 (extract_element_attribute implementation)
- **Issue:** `std::str::from_utf8(e.local_name().as_ref())` creates a temporary LocalName that is freed before the borrow used in `if local == element_name` — compiler E0716
- **Fix:** Bound `let local_bytes = e.local_name()` before calling `.as_ref()` on it; same for attr key
- **Files modified:** src/service/ptz.rs
- **Verification:** cargo build passes
- **Committed in:** f12984e (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 bugs)
**Impact on plan:** Both fixes required for compilation. No scope changes.

## Issues Encountered

- PROFILE_TOKEN imported initially then found unused in ptz.rs (PTZ handler has no xaddr, no need for profile constant at handler level) — removed the import cleanly before commit

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- PTZServiceHandler ready for wiring into server.rs router in Phase 5
- Follows identical mount pattern as DeviceServiceHandler and MediaServiceHandler
- PTZ service mounts at `/onvif/ptz_service` via Router::merge() in run()
- All 15 PTZ-XX requirements satisfied; TEST-01 (frigate_compat.rs) and TEST-02 (virtual_ptz example) remain for later plans

---
*Phase: 04-ptz-service*
*Completed: 2026-04-05*
