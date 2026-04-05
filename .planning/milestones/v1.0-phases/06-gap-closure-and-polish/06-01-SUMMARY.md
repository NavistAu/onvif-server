---
phase: 06-gap-closure-and-polish
plan: 01
subsystem: infra
tags: [onvif, xaddr, wssecurity, auth, ptz, http-integration-test]

# Dependency graph
requires:
  - phase: 05-imaging-events-discovery-and-polish
    provides: server.rs with all 5 services wired, run() in place, #[ignore] auth test stubs
provides:
  - advertised_host builder field on OnvifServerBuilder propagated to all XAddr strings
  - HTTP-level WS-Security PasswordText auth integration tests (valid + invalid)
  - PTZServiceHandler constructed via ::new() in server.rs
affects: [consumer-integration, wsdiscovery, frigate-compat, odm-smoke]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "advertised_host defaults to 0.0.0.0 for backward compat; callers set routable IP for real clients"
    - "HTTP-level auth tests use raw TcpStream POST + PasswordText WS-Security header (no digest complexity)"

key-files:
  created: []
  modified:
    - src/server.rs
    - tests/device_management.rs

key-decisions:
  - "PasswordText used in auth tests (not PasswordDigest) — avoids timestamp freshness and nonce replay issues in deterministic test environment"
  - "advertised_host defaults to '0.0.0.0' for backward compatibility with existing users"
  - "PTZServiceHandler::new() used instead of struct literal for forward compatibility with future field additions"

patterns-established:
  - "XAddr construction: all 6 format strings in run() use self.advertised_host instead of literal 0.0.0.0"
  - "HTTP auth tests: real TcpListener on random port, raw HTTP POST, read full response to Vec<u8>"

requirements-completed: [DEV-02, DEV-03, DEV-05, DISC-01, INFRA-07, PTZ-01]

# Metrics
duration: 8min
completed: 2026-04-05
---

# Phase 6 Plan 1: Gap Closure — Advertised Host, HTTP Auth Tests, PTZ Constructor Summary

**Routable XAddr support via advertised_host builder field, HTTP-level WS-Security PasswordText auth tests replacing #[ignore] stubs, and PTZServiceHandler::new() constructor fix — all 56 tests pass**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-04-05T13:00:00Z
- **Completed:** 2026-04-05T13:08:00Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Added `advertised_host` field and builder method to `OnvifServerBuilder` (defaults to "0.0.0.0", set to routable IP for real ONVIF clients like Frigate/ODM/Home Assistant)
- Replaced all 6 hardcoded "0.0.0.0" XAddr format strings in `run()` with `self.advertised_host`; also fixed the discovery block's `disc_xaddr`
- Replaced two `#[ignore]` `todo!()` stub tests with real HTTP-level WS-Security PasswordText integration tests: valid creds return HTTP 200 + `GetDeviceInformationResponse`, invalid creds return non-200 or Fault body
- Fixed `PTZServiceHandler { svc: ptz_svc }` struct literal to `PTZServiceHandler::new(ptz_svc)` for forward compatibility
- Full test suite: 56 tests pass, 0 failures, 0 ignored

## Task Commits

Each task was committed atomically:

1. **Task 1: Add advertised_host to OnvifServerBuilder and use it in run()** - `6f6c98c` (feat)
2. **Task 2: Write HTTP-level auth integration tests** - `65c0f26` (feat)
3. **Task 3: Verify full test suite remains green** - (no new code; verified via `cargo test`)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src/server.rs` - Added `advertised_host: String` to `OnvifServerBuilder` and `OnvifServer`; added `advertised_host()` builder method; replaced 6 hardcoded XAddr strings; fixed PTZ constructor
- `tests/device_management.rs` - Replaced two `#[ignore]` stubs with `device_auth_valid_credential` and `device_auth_invalid_credential` HTTP integration tests

## Decisions Made

- **PasswordText over PasswordDigest in tests:** Avoids timestamp freshness windows and nonce replay rules; soap-server's auth callback accepts PasswordText; test environment stays deterministic
- **Default advertised_host = "0.0.0.0":** Backward compatible; existing callers without `.advertised_host()` continue to work identically
- **PTZServiceHandler::new():** Struct literal was fragile to future field additions; `::new()` is the established pattern for all other service handlers

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All three v1.0-MILESTONE-AUDIT.md gap items are now closed
- `OnvifServer::builder().advertised_host("192.168.1.10").port(8080)` compiles and propagates the routable address into all XAddrs
- Real ONVIF clients (Frigate, ODM, Home Assistant) can connect via the configured routable address
- Auth enforcement is verified at the HTTP layer with automated tests

---
*Phase: 06-gap-closure-and-polish*
*Completed: 2026-04-05*

## Self-Check: PASSED

- FOUND: src/server.rs
- FOUND: tests/device_management.rs
- FOUND: 06-01-SUMMARY.md
- FOUND commit: 6f6c98c (feat: advertised_host + PTZ fix)
- FOUND commit: 65c0f26 (feat: HTTP auth integration tests)
