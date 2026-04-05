---
phase: 05-imaging-events-discovery-and-polish
plan: 02
subsystem: api
tags: [onvif, imaging, events, discovery, ws-discovery, soap, rust, server-wiring]

# Dependency graph
requires:
  - phase: 05-01
    provides: ImagingServiceHandler and EventServiceHandler built in plan 01

provides:
  - run() wires all 5 services (Device, Media, PTZ, Imaging, Events) into axum router
  - DeviceServiceHandler::new() accepts 5 xaddrs; GetCapabilities and GetServices advertise all 5 services
  - WS-Discovery UDP task behind cfg(feature="discovery") responds to Probe with ProbeMatch
  - virtual_ptz example implements ImagingService and EventService; registers all 5 services
  - ODM smoke test: 6 integration tests covering full first-connect sequence

affects:
  - downstream consumers that call DeviceServiceHandler::new() (signature changed to 6 args)

# Tech tracking
tech-stack:
  added:
    - socket2 0.5 (optional, gated behind discovery feature)
    - WSDL stubs: wsn-bw2.wsdl, wsrf-rw2.wsdl, ws-addr.xsd, wsn-t1.xsd
  patterns:
    - EmbeddedWsdlLoader extended with 4 new stubs for OASIS/W3C imports in events.wsdl
    - WS-Discovery: UDP multicast on 3702, responds only to messages containing "Probe" bytes
    - DeviceServiceHandler carries 5 xaddr fields, formats GetCapabilities sections per ONVIF spec

key-files:
  created:
    - tests/odm_smoke.rs
    - src/discovery.rs
    - wsdl/wsn-bw2.wsdl
    - wsdl/wsrf-rw2.wsdl
    - wsdl/ws-addr.xsd
    - wsdl/wsn-t1.xsd
  modified:
    - src/service/device.rs
    - src/server.rs
    - src/lib.rs
    - examples/virtual_ptz.rs
    - src/wsdl_loader.rs
    - tests/device_management.rs

key-decisions:
  - "DeviceServiceHandler::new() takes 5 xaddrs — struct has media_xaddr, ptz_xaddr, imaging_xaddr, events_xaddr fields alongside xaddr"
  - "imaging_service and event_service are required at run() time via ok_or() — consistent with device/media/ptz pattern"
  - "Discovery spawned via tokio::spawn after router assembly but before axum::serve; errors logged to stderr via eprintln"
  - "EmbeddedWsdlLoader needed 4 new stubs (bw-2.wsdl, rw-2.wsdl, ws-addr.xsd, t-1.xsd) for events.wsdl WSDL/XSD imports"

patterns-established:
  - "5-service GetCapabilities: separate tt:Device/tt:Media/tt:PTZ/tt:Imaging/tt:Events sections with XAddr children"
  - "5-service GetServices: 5 tds:Service blocks with Namespace, XAddr, and Version elements"

requirements-completed: [DISC-01, DISC-02, TEST-03]

# Metrics
duration: ~7min
completed: 2026-04-05
---

# Phase 5 Plan 02: Server Wiring, WS-Discovery, and ODM Smoke Test Summary

**All 5 ONVIF services wired into run() with 5-xaddr DeviceServiceHandler; WS-Discovery UDP task behind feature flag; ODM smoke test with 6 green integration tests**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-04-05T12:42:28Z
- **Completed:** 2026-04-05T12:49:30Z
- **Tasks:** 3 (including TDD Red/Green for tasks 1 and 3)
- **Files modified:** 10

## Accomplishments
- DeviceServiceHandler extended with media_xaddr, ptz_xaddr, imaging_xaddr, events_xaddr fields; GetCapabilities returns all 5 service sections per ONVIF spec; GetServices lists all 5 namespaces
- run() now requires all 5 services and builds 5 soap_server::ServerBuilder chains merged into a single axum router
- discovery.rs: async UDP listener on multicast 3702 responds to Probe messages with ProbeMatch; compiled only under cfg(feature="discovery")
- virtual_ptz example extended with ImagingService and EventService impls and builder registrations
- ODM smoke test: 6 tests all pass — GetCapabilities, GetDeviceInformation, GetServices, GetImagingSettings, event lifecycle (Create/Pull/Unsubscribe), and full 7-step ODM sequence

## Task Commits

Each task was committed atomically:

1. **Task 1: Update DeviceServiceHandler + wire Imaging/Events in run()** - `508bbd2` (feat)
2. **Task 2: WS-Discovery UDP task + virtual_ptz extensions** - `153a21e` (feat)
3. **Task 3: ODM smoke test** - included in Task 1 commit (TDD: test written first, passes in same commit)
4. **Auto-fix: WSDL stub additions for events.wsdl imports** - `73dc070` (fix)

## Files Created/Modified
- `src/service/device.rs` - DeviceServiceHandler with 5 xaddr fields; 5-service GetCapabilities and GetServices
- `src/server.rs` - run() wires all 5 services; discovery spawn block
- `src/lib.rs` - exports ImagingServiceHandler, EventServiceHandler, ImagingSettings; discovery module declaration
- `src/discovery.rs` - run_discovery async fn behind cfg(feature="discovery")
- `src/wsdl_loader.rs` - 4 new stubs for OASIS/W3C schemas imported by events.wsdl
- `examples/virtual_ptz.rs` - ImagingService and EventService impls; .imaging_service() and .event_service() in builder
- `tests/odm_smoke.rs` - 6 ODM integration tests (full first-connect sequence)
- `tests/device_management.rs` - updated for 6-arg DeviceServiceHandler::new; added ImagingService/EventService stubs
- `wsdl/wsn-bw2.wsdl` - OASIS WS-BaseNotification WSDL stub
- `wsdl/wsrf-rw2.wsdl` - OASIS WS-ResourceFramework WSDL stub
- `wsdl/ws-addr.xsd` - W3C WS-Addressing schema stub
- `wsdl/wsn-t1.xsd` - OASIS WS-Notification topics schema stub

## Decisions Made
- DeviceServiceHandler::new() now takes 6 arguments (svc + 5 xaddrs) — existing tests in device_management.rs updated accordingly
- imaging_service and event_service are required at run() time via ok_or(), consistent with the existing 3-service pattern
- Discovery task uses tokio::spawn without join handle — errors are printed to stderr and the main server continues
- WS-Discovery Probe detection uses a simple byte scan for "Probe" rather than full XML parsing — lightweight and sufficient for ONVIF clients
- 4 WSDL/XSD stubs were needed for events.wsdl's external OASIS/W3C imports — EmbeddedWsdlLoader extended to serve them

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing WSDL/XSD stubs for events.wsdl imports**
- **Found during:** Task 2 verification (full test suite run)
- **Issue:** events.wsdl imports bw-2.wsdl, rw-2.wsdl, ws-addr.xsd, t-1.xsd from external OASIS/W3C URLs; EmbeddedWsdlLoader returned "Unknown WSDL/XSD" for each; device_server_binds_and_serves_auth_exempt_op panicked at ServerBuilder::build
- **Fix:** Created 4 minimal stub files in wsdl/ and added 4 entries to EmbeddedWsdlLoader match block
- **Files modified:** src/wsdl_loader.rs, wsdl/wsn-bw2.wsdl, wsdl/wsrf-rw2.wsdl, wsdl/ws-addr.xsd, wsdl/wsn-t1.xsd
- **Commit:** 73dc070

## Issues Encountered
None beyond the auto-fixed WSDL stub gap.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- All 5 ONVIF services are fully wired and reachable over HTTP
- WS-Discovery is ready for LAN testing with the `discovery` feature flag
- Full ODM smoke test suite validates the complete first-connect sequence
- Phase 5 is complete — all plans executed

---
*Phase: 05-imaging-events-discovery-and-polish*
*Completed: 2026-04-05*

## Self-Check: PASSED

- src/service/device.rs: FOUND
- src/server.rs: FOUND
- src/lib.rs: FOUND
- src/discovery.rs: FOUND
- tests/odm_smoke.rs: FOUND
- examples/virtual_ptz.rs: FOUND
- 05-02-SUMMARY.md: FOUND
- Commit 508bbd2 (Task 1): FOUND
- Commit 153a21e (Task 2): FOUND
- Commit 73dc070 (Auto-fix): FOUND
