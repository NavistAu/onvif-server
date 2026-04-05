---
phase: 06-gap-closure-and-polish
verified: 2026-04-05T13:30:00Z
status: passed
score: 5/5 must-haves verified
re_verification: false
---

# Phase 6: Gap Closure and Polish — Verification Report

**Phase Goal:** Close all audit gaps — advertised host for real client connectivity, HTTP-level auth tests, and constructor consistency
**Verified:** 2026-04-05T13:30:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | GetCapabilities and GetServices XAddrs contain the consumer-configured advertised host, not 0.0.0.0 | VERIFIED | `src/server.rs` lines 66-70: all 5 service XAddr format strings use `self.advertised_host` — `format!("http://{}:{}/onvif/device_service", self.advertised_host, self.port)` etc. No hardcoded `0.0.0.0` in XAddr construction. |
| 2 | WS-Discovery ProbeMatch XAddr contains the consumer-configured advertised host | VERIFIED | `src/server.rs` line 179: discovery block uses `format!("http://{}:{}/onvif/device_service", self.advertised_host, self.port)` for `disc_xaddr`. |
| 3 | An HTTP-level test with valid WS-Security PasswordText credentials returns HTTP 200 | VERIFIED | `tests/device_management.rs` lines 300-362: `device_auth_valid_credential` — no `#[ignore]`, no `todo!()`. Sends real HTTP POST with PasswordText WS-Security header; asserts `response_str.starts_with("HTTP/1.1 200")` and `contains("GetDeviceInformationResponse")`. |
| 4 | An HTTP-level test with invalid credentials receives a SOAP fault response (non-200 or body contains Fault) | VERIFIED | `tests/device_management.rs` lines 364-422: `device_auth_invalid_credential` — no `#[ignore]`, no `todo!()`. Uses `wrongpassword`; asserts `!response_str.starts_with("HTTP/1.1 200") \|\| response_str.contains("Fault")`. |
| 5 | PTZServiceHandler is constructed via ::new() in server.rs | VERIFIED | `src/server.rs` line 82: `let ptz_handler = PTZServiceHandler::new(ptz_svc);`. No struct literal form found anywhere in `src/`. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/server.rs` | `advertised_host` field on `OnvifServerBuilder`, used in XAddr construction in `run()` | VERIFIED | Field present on both `OnvifServer` (line 34) and `OnvifServerBuilder` (line 208). Builder method `advertised_host()` at line 241. Default `"0.0.0.0"` in `new()` at line 228. Propagated through `build()` at line 306. |
| `tests/device_management.rs` | HTTP-level auth tests (valid + invalid credentials) | VERIFIED | Both `device_auth_valid_credential` (line 300) and `device_auth_invalid_credential` (line 364) are present, substantive (real TcpListener, raw HTTP POST, response assertion), and not `#[ignore]`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `OnvifServerBuilder::advertised_host()` | `OnvifServer::run()` XAddr construction | `self.advertised_host` field replacing hardcoded `0.0.0.0` | WIRED | `advertised_host()` sets `self.advertised_host` (line 242); `build()` passes it to `OnvifServer` (line 306); `run()` uses `self.advertised_host` in all 6 XAddr format strings (lines 66-70, 179). |
| `tests/device_management.rs` | `OnvifServer::run()` | Real `TcpListener` on random port, raw HTTP POST with WS-Security header | WIRED | Both auth tests bind `127.0.0.1:0`, drop listener, build server, spawn `server.run()`, then connect via `TcpStream` with a full SOAP envelope containing `wsse:UsernameToken`. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| DEV-02 | 06-01-PLAN.md | User can call GetCapabilities and receive XAddrs for all registered services | SATISFIED | All 5 service XAddrs in `run()` now use `self.advertised_host` instead of literal `0.0.0.0`. Partial status from audit is now closed. |
| DEV-03 | 06-01-PLAN.md | User can call GetServices and receive service namespace, XAddr, and capabilities | SATISFIED | Same fix as DEV-02 — `xaddr` and service xaddrs passed to `DeviceServiceHandler::new()` all come from `self.advertised_host`. |
| DEV-05 | 06-01-PLAN.md | Auth wiring tested at HTTP level (audit gap: `#[ignore]` stubs) | SATISFIED | Both auth integration tests (`device_auth_valid_credential`, `device_auth_invalid_credential`) are now real, non-ignored, non-stub tests that exercise auth at the HTTP layer. |
| DISC-01 | 06-01-PLAN.md | WS-Discovery ProbeMatch XAddr uses advertised host | SATISFIED | `disc_xaddr` in the `#[cfg(feature = "discovery")]` block at line 179 uses `self.advertised_host`. |
| INFRA-07 | 06-01-PLAN.md | WS-Security UsernameToken authentication verified at HTTP layer | SATISFIED | HTTP-level auth tests confirm soap-server's auth callback: valid PasswordText returns 200; invalid PasswordText returns non-200 or Fault. |
| PTZ-01 | 06-01-PLAN.md | PTZServiceHandler constructed consistently via `::new()` | SATISFIED | `src/server.rs` line 82 uses `PTZServiceHandler::new(ptz_svc)`. No struct literal form exists anywhere in `src/`. |

**Requirement ID note:** REQUIREMENTS.md lists DEV-05 as "User can call GetScopes and receive ONVIF-standard scope URIs". However, the v1.0-MILESTONE-AUDIT.md uses DEV-05 to label the auth-test gap ("Auth wiring is present in run() but both auth integration tests are #[ignore]"). The GetScopes operation itself was already working and tested (test `device_get_scopes` at line 166 of `tests/device_management.rs`). Phase 6 closes the auth-test gap that the audit attributed to DEV-05. No orphaned requirements detected — all 6 IDs are accounted for with implementation evidence.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `tests/device_management.rs` | 3 | Comment references `#[ignore]` ("Wave 0: stubs compile; #[ignore] removed...") | Info | Comment only — historical note; no actual `#[ignore]` attribute in the file |

No blocker or warning-level anti-patterns found. No `todo!()`, `#[ignore]`, `return null`, or placeholder patterns in either modified file.

### Human Verification Required

None. All must-have truths are verifiable programmatically from the codebase:

- XAddr format strings are inspectable directly in source.
- Auth tests are fully implemented with real HTTP communication patterns — no `#[ignore]` or `todo!()`.
- PTZ constructor form is statically readable.

The only items that would benefit from human verification are runtime behaviors (e.g., confirming a real Frigate or ODM client connects via the configured `advertised_host`), but these are not required to verify phase goal achievement.

### Gaps Summary

No gaps. All five observable truths are verified. All artifacts exist, are substantive, and are correctly wired. All six requirement IDs from the PLAN are satisfied with implementation evidence. No anti-pattern blockers.

---

_Verified: 2026-04-05T13:30:00Z_
_Verifier: Claude (gsd-verifier)_
