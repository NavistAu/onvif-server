---
phase: 4
slug: ptz-service
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-05
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | tokio::test (existing pattern) |
| **Config file** | none — `[dev-dependencies]` in Cargo.toml |
| **Quick run command** | `cargo test -p onvif-server ptz` |
| **Full suite command** | `cargo test -p onvif-server` |
| **Estimated runtime** | ~20 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p onvif-server`
- **After every plan wave:** Run `cargo test -p onvif-server`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 25 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 04-01-T1 | 04-01 | 1 | PTZ-01..06 | unit | `cargo test -p onvif-server ptz_get_nodes ptz_get_service_capabilities` | ✅ W0 | ⬜ pending |
| 04-01-T2 | 04-01 | 1 | PTZ-07..10 | unit | `cargo test -p onvif-server ptz_relative_move ptz_get_status` | ✅ W0 | ⬜ pending |
| 04-01-T3 | 04-01 | 1 | PTZ-11..15 | unit | `cargo test -p onvif-server ptz_get_presets ptz_goto_preset` | ✅ W0 | ⬜ pending |
| 04-02-T1 | 04-02 | 2 | PTZ wiring | build | `cargo build -p onvif-server` | ✅ W0 | ⬜ pending |
| 04-02-T2 | 04-02 | 2 | TEST-01 | integration | `cargo test -p onvif-server frigate_autotracker` | ✅ W0 | ⬜ pending |
| 04-02-T3 | 04-02 | 2 | TEST-02 | smoke | `cargo build --example virtual_ptz` | ✅ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 artifacts created within the first plan's initial tasks:

- [x] `tests/ptz_service.rs` — unit tests for PTZ-01 through PTZ-15 with `#[ignore]`
- [x] `tests/frigate_compat.rs` — Frigate call sequence test (TEST-01) with `#[ignore]`
- [x] `examples/virtual_ptz.rs` — example binary (TEST-02)
- [x] PTZ types expanded in `src/generated/types.rs`

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify — every task has a concrete `cargo` command
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags — all commands are one-shot
- [x] Feedback latency < 25s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete
