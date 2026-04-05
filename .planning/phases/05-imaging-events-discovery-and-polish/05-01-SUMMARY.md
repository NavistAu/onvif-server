---
phase: 05-imaging-events-discovery-and-polish
plan: 01
subsystem: api
tags: [onvif, imaging, events, ws-basenotification, soap, rust, quick-xml, uuid, chrono]

# Dependency graph
requires:
  - phase: 04-ptz-service
    provides: PTZServiceHandler pattern — extract_local_name, extract_text_element, SoapHandler dispatch

provides:
  - ImagingServiceHandler implementing SoapHandler with GetImagingSettings dispatch
  - ImagingSettings type with Option<f32> fields for brightness, contrast, sharpness, color_saturation, white_balance
  - EventServiceHandler implementing SoapHandler with Arc<Mutex<HashMap>> subscription state
  - Full pull-point subscription lifecycle: CreatePullPointSubscription, PullMessages, Unsubscribe
  - 12 new green unit tests (5 imaging, 7 events)

affects:
  - 05-02 (server wiring — will register ImagingServiceHandler and EventServiceHandler at run time)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - ImagingSettings serializes only Some fields — None fields produce no XML element
    - EventServiceHandler holds Arc<Mutex<HashMap<String, SubscriptionInfo>>> for subscription state
    - Subscription lifecycle: Create inserts UUID key, Pull reads termination_time, Unsubscribe removes key
    - get_event_properties is handler-internal static response; trait method exists for future extensibility only

key-files:
  created:
    - src/service/imaging.rs
    - src/service/events.rs
    - tests/imaging_service.rs
    - tests/events_service.rs
  modified:
    - src/traits/imaging.rs
    - src/traits/events.rs
    - src/generated/types.rs
    - src/generated/mod.rs
    - src/service/mod.rs

key-decisions:
  - "ImagingSettings serializes Option<f32> fields only when Some — no stray empty XML elements"
  - "EventService trait has only get_event_properties; CreatePullPointSubscription/PullMessages/Unsubscribe are handler-internal state operations"
  - "SubscriptionInfo holds only termination_time — UUID key is the subscription identifier"
  - "PullMessages returns SoapFault::sender for unknown subscription IDs; Unsubscribe is idempotent"
  - "get_event_properties trait method returns Result<(), OnvifError> — handler always returns static XML regardless"

patterns-established:
  - "Option<f32> field serialization: emit element only when Some, format as integer via `v as i32`"
  - "Subscription state: Arc<Mutex<HashMap<String, SubscriptionInfo>>> with UUID keys from uuid::Uuid::new_v4()"

requirements-completed: [IMG-01, EVT-01, EVT-02, EVT-03, EVT-04]

# Metrics
duration: 18min
completed: 2026-04-05
---

# Phase 5 Plan 01: Imaging + Events Service Handlers Summary

**ImagingServiceHandler with typed ImagingSettings type and EventServiceHandler with UUID-keyed pull-point subscription map behind Arc<Mutex<_>>**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-04-05T12:21:00Z
- **Completed:** 2026-04-05T12:39:51Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- ImagingSettings type added with 6 Option<f32> fields; only Some fields emit XML elements
- ImagingServiceHandler dispatches GetImagingSettings to trait, returns typed XML with namespace declarations
- EventServiceHandler holds full in-memory subscription state; full lifecycle (Create/Pull/Unsubscribe) verified by 7 tests
- EventService trait slimmed to single get_event_properties method — subscription operations are handler-internal

## Task Commits

Each task was committed atomically:

1. **Task 1: ImagingService typed trait + ImagingSettings type + ImagingServiceHandler** - `3db7e7c` (feat)
2. **Task 2: EventService typed trait + EventServiceHandler with subscription state** - `aa79ef2` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/service/imaging.rs` - ImagingServiceHandler with GetImagingSettings dispatch
- `src/service/events.rs` - EventServiceHandler with subscription HashMap state
- `src/traits/imaging.rs` - Updated get_imaging_settings signature with video_source_token param and ImagingSettings return
- `src/traits/events.rs` - Slimmed to get_event_properties only; subscription ops removed
- `src/generated/types.rs` - Added ImagingSettings struct with Option<f32> fields
- `src/generated/mod.rs` - Added ImagingSettings to public re-exports
- `src/service/mod.rs` - Added pub mod imaging and pub mod events
- `tests/imaging_service.rs` - 5 tests: response element, ImagingSettings element, brightness value, none fields, unknown op
- `tests/events_service.rs` - 7 tests: GetEventProperties, CreatePullPointSubscription x2, PullMessages, Unsubscribe, unknown subscription fault, state verification

## Decisions Made
- ImagingSettings serializes as `v as i32` for integer display (50.0 -> `<tt:Brightness>50</tt:Brightness>`)
- EventService trait method `get_event_properties` returns `Result<(), OnvifError>` — handler ignores it and returns static XML (same GetCapabilities pattern from DeviceServiceHandler)
- PullMessages for unknown subscription returns `SoapFault::sender` not a typed fault — consistent with codebase pattern
- Unsubscribe is idempotent — removes if present, ignores if absent; never errors on missing key

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- ImagingServiceHandler and EventServiceHandler exist and pass all tests
- Both handlers need to be registered in OnvifServerBuilder (Phase 5 Plan 02)
- lib.rs needs to re-export ImagingServiceHandler, EventServiceHandler, and ImagingSettings

---
*Phase: 05-imaging-events-discovery-and-polish*
*Completed: 2026-04-05*

## Self-Check: PASSED

- src/service/imaging.rs: FOUND
- src/service/events.rs: FOUND
- tests/imaging_service.rs: FOUND
- tests/events_service.rs: FOUND
- 05-01-SUMMARY.md: FOUND
- Commit 3db7e7c (Task 1): FOUND
- Commit aa79ef2 (Task 2): FOUND
