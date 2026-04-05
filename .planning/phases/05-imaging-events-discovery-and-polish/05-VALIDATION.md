---
phase: 5
slug: imaging-events-discovery-and-polish
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-05
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio-test |
| **Config file** | none — Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test -p onvif-server` |
| **Full suite command** | `cargo test -p onvif-server --features discovery` |
| **Estimated runtime** | ~25 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p onvif-server`
- **After every plan wave:** Run `cargo test -p onvif-server --features discovery`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 05-01-T1 | 05-01 | 1 | IMG-01 | unit | `cargo test -p onvif-server imaging_get_imaging_settings` | ✅ W0 | ⬜ pending |
| 05-01-T2 | 05-01 | 1 | EVT-01..04 | unit | `cargo test -p onvif-server events_` | ✅ W0 | ⬜ pending |
| 05-02-T1 | 05-02 | 2 | wiring | build | `cargo build -p onvif-server` | ✅ W0 | ⬜ pending |
| 05-02-T2 | 05-02 | 2 | DISC-01, DISC-02 | unit | `cargo test -p onvif-server --features discovery discovery_` | ✅ W0 | ⬜ pending |
| 05-02-T3 | 05-02 | 2 | TEST-03 | integration | `cargo test -p onvif-server odm_smoke` | ✅ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 artifacts created within the first plan's tasks:

- [x] `tests/imaging_service.rs` — test stubs for IMG-01
- [x] `tests/events_service.rs` — test stubs for EVT-01 through EVT-04
- [x] `tests/odm_smoke.rs` — ODM call sequence test (TEST-03)
- [x] Types expanded in `src/generated/types.rs`

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify — every task has a concrete `cargo` command
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags — all commands are one-shot
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete
