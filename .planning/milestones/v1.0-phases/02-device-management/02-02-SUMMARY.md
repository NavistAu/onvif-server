---
phase: 02-device-management
plan: "02"
subsystem: api
tags: [rust, onvif, soap, axum, tokio, wsdl]

requires:
  - phase: 02-device-management
    plan: "01"
    provides: "DeviceServiceHandler skeleton with 4 operations; 3 stubs returning ActionNotSupported"

provides:
  - "Complete DeviceServiceHandler — all 7 Device Management operations implemented"
  - "OnvifServer::run() binding port via tokio TcpListener and axum::serve"
  - "soap-server ServerBuilder wired: from_wsdl_bytes_with_loader, path, default_handler, auth, auth_bypass"
  - "EmbeddedWsdlLoader extended with xmlmime, soap-envelope, wsn-b2, xop-include stubs"
  - "Integration test verifying GetSystemDateAndTime auth-bypass over real HTTP"
  - "8 passing device management tests (DEV-01 through DEV-07 + server bind test)"

affects:
  - "03-media: same Arc<dyn Service> + SoapHandler pattern, same run() wiring pattern"
  - "Phase 5 ODM smoke test: uses run() to start server for end-to-end ODM validation"

tech-stack:
  added:
    - "axum = 0.8 (direct dependency — needed for axum::serve in run())"
    - "tokio features: rt, net added (required for TcpListener and async runtime)"
  patterns:
    - "OnvifServer::run() constructs handler, builds soap_svc via ServerBuilder, serves via axum"
    - "EmbeddedWsdlLoader handles all external WSDL/XSD imports — no network access at startup"
    - "Auth tests requiring WS-Security digest deferred to Phase 5 ODM smoke test"

key-files:
  created:
    - wsdl/xmlmime.xsd
    - wsdl/soap-envelope.xsd
    - wsdl/wsn-b2.xsd
    - wsdl/xop-include.xsd
  modified:
    - src/service/device.rs
    - src/server.rs
    - src/wsdl_loader.rs
    - Cargo.toml
    - tests/device_management.rs

key-decisions:
  - "axum added as direct dependency (was only transitive via soap-server) — required for axum::serve in run()"
  - "Auth tests (device_auth_valid/invalid_credential) remain #[ignore]: WS-Security PasswordDigest construction requires nonce+SHA-1 and is tested end-to-end via ODM in Phase 5"
  - "EmbeddedWsdlLoader extended with W3C/OASIS stub XSDs — onvif.xsd imports xmlmime, soap-envelope, wsn-b2, xop/include which must resolve at ServerBuilder::build time"

metrics:
  duration: "~6 min"
  completed: "2026-04-05"
  tasks: 2
  files_modified: 9
---

# Phase 2 Plan 02: Complete Device Management and Server Binding Summary

**OnvifServer::run() wired to soap-server with all 7 Device Management operations functional; GetSystemDateAndTime verified auth-exempt over real HTTP**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-04-05T08:05:52Z
- **Completed:** 2026-04-05T08:11:47Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- GetScopes, GetHostname, GetNetworkInterfaces stub methods replaced with full XML-building implementations
- All 7 Device Management operations now return spec-compliant ONVIF XML with tds/tt namespace declarations
- OnvifServer::run() implemented — binds TcpListener, constructs DeviceServiceHandler, wires soap-server ServerBuilder, starts axum::serve
- Auth bypass correctly wired — GetSystemDateAndTime returns HTTP 200 without WS-Security header (confirmed by real HTTP test)
- EmbeddedWsdlLoader extended with 4 external XSD stubs required by onvif.xsd imports

## Task Commits

1. **Task 1: Complete GetScopes, GetHostname, GetNetworkInterfaces** - `722a415` (feat)
2. **Task 2: OnvifServer::run(), tokio features, auth integration tests** - `08ab739` (feat)

## Files Created/Modified

- `src/service/device.rs` - Replaced 3 stub methods with full XML implementations
- `src/server.rs` - Added OnvifServer::run() with full soap-server wiring
- `src/wsdl_loader.rs` - Added match arms for xmlmime, soap-envelope, b-2.xsd, include
- `Cargo.toml` - Added axum = "0.8"; added rt, net to tokio features
- `tests/device_management.rs` - Activated 3 handler tests; added server binding test; updated auth test comments
- `wsdl/xmlmime.xsd` - W3C XML MIME stub schema
- `wsdl/soap-envelope.xsd` - W3C SOAP envelope stub schema
- `wsdl/wsn-b2.xsd` - OASIS WS-BaseNotification stub schema
- `wsdl/xop-include.xsd` - W3C XOP Include stub schema

## Decisions Made

- axum added as direct dependency — was only transitive via soap-server but is needed explicitly for `axum::serve` in `run()`.
- WS-Security digest auth tests (device_auth_valid_credential, device_auth_invalid_credential) remain `#[ignore]` — constructing a valid PasswordDigest token requires nonce generation, base64 encoding, and SHA-1 hashing; this complexity is deferred to the end-to-end ODM smoke test in Phase 5.
- EmbeddedWsdlLoader extended with stubs for all 4 external schemas imported by `onvif.xsd` — required for `ServerBuilder::build()` to succeed without network access.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Arc<DeviceServiceHandler> does not implement SoapHandler**
- **Found during:** Task 2 (implementing run())
- **Issue:** Plan showed `Arc::new(handler)` passed to `default_handler()`, but `SoapHandler` is not implemented for `Arc<T>` in soap-server
- **Fix:** Pass `DeviceServiceHandler` directly (already Clone-equivalent via inner Arc<dyn DeviceService>)
- **Files modified:** src/server.rs
- **Committed in:** 08ab739 (Task 2 commit)

**2. [Rule 3 - Blocking] EmbeddedWsdlLoader missing xmlmime, soap-envelope, wsn-b2, xop-include stubs**
- **Found during:** Task 2 (running server binding test)
- **Issue:** onvif.xsd imports 4 external XSD schemas via absolute URLs; ServerBuilder::build() calls EmbeddedWsdlLoader for each import; all 4 failed with "Unknown WSDL/XSD"
- **Fix:** Created 4 minimal stub XSD files in wsdl/; added 4 match arms to EmbeddedWsdlLoader using rsplit('/') filename extraction
- **Files modified:** src/wsdl_loader.rs, wsdl/xmlmime.xsd, wsdl/soap-envelope.xsd, wsdl/wsn-b2.xsd, wsdl/xop-include.xsd
- **Committed in:** 08ab739 (Task 2 commit)

**3. [Rule 2 - Missing critical] device_service_arc() method referenced in test doesn't exist**
- **Found during:** Task 2 RED phase (compile check)
- **Issue:** Test used non-existent method device_service_arc(Arc<dyn DeviceService>); the builder only has device_service(impl DeviceService + 'static)
- **Fix:** Changed test to call device_service(TestDevice {...}) directly — TestDevice implements DeviceService
- **Files modified:** tests/device_management.rs
- **Committed in:** 08ab739 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 Rule 1, 1 Rule 3, 1 Rule 2)
**Impact on plan:** All fixes required for compilation and runtime correctness. No scope creep.

## Issues Encountered

None beyond the auto-fixed deviations above.

## Next Phase Readiness

- Phase 3 (Media Service) can immediately follow the same Arc<dyn Service> + SoapHandler + run() pattern
- EmbeddedWsdlLoader is now fully capable of resolving all ONVIF WSDL imports at startup
- OnvifServer::run() is tested and functional — Phase 5 ODM test can call it directly

---
*Phase: 02-device-management*
*Completed: 2026-04-05*

## Self-Check: PASSED

- FOUND: src/service/device.rs
- FOUND: src/server.rs
- FOUND: wsdl/xmlmime.xsd
- FOUND: .planning/phases/02-device-management/02-02-SUMMARY.md
- Commit 722a415: FOUND (Task 1 — GetScopes/GetHostname/GetNetworkInterfaces)
- Commit 08ab739: FOUND (Task 2 — OnvifServer::run() and server binding test)
