---
phase: 2
slug: device-management
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-05
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio `#[tokio::test]` |
| **Config file** | none — `cargo test` discovers all `#[test]` and `#[tokio::test]` |
| **Quick run command** | `cargo test --package onvif-server` |
| **Full suite command** | `cargo test --package onvif-server` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --package onvif-server`
- **After every plan wave:** Run `cargo test --package onvif-server`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 20 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01-T1 | 02-01 | 1 | DEV-01 | integration | `cargo test device_get_system_date_and_time` | ✅ W0 (created in first plan) | ⬜ pending |
| 02-01-T2 | 02-01 | 1 | DEV-02, DEV-03 | integration | `cargo test device_get_capabilities_xaddr device_get_services` | ✅ W0 | ⬜ pending |
| 02-01-T3 | 02-01 | 1 | DEV-04 | integration | `cargo test device_get_device_information` | ✅ W0 | ⬜ pending |
| 02-02-T1 | 02-02 | 2 | DEV-05, DEV-06, DEV-07 | integration | `cargo test device_get_scopes device_get_hostname device_get_network_interfaces` | ✅ W0 | ⬜ pending |
| 02-02-T2 | 02-02 | 2 | AUTH | integration | `cargo test device_auth_valid_credential device_auth_invalid_credential` | ✅ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 artifacts are created within the first plan's initial task:

- [x] `tests/device_management.rs` — integration tests for DEV-01 through DEV-07 + auth (stubs with `#[ignore]`)
- [x] Expanded request/response types in `src/generated/types.rs` for Device Management operations

*Test stubs compile immediately; implementations remove `#[ignore]` as they're built.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify — every task has a concrete `cargo test` command
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references — test stubs created in first plan
- [x] No watch-mode flags — all commands are one-shot
- [x] Feedback latency < 20s — `cargo test` ~15s on warm cache
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete
