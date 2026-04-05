---
phase: 1
slug: foundation
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-05
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | none — `[dev-dependencies]` in Cargo.toml |
| **Quick run command** | `cargo build -p onvif-server` |
| **Full suite command** | `cargo test -p onvif-server` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo build -p onvif-server`
- **After every plan wave:** Run `cargo test -p onvif-server`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 01-01-T1 | 01-01 | 1 | INFRA-01 | build | `cargo build -p onvif-server` | ✅ W0 (created in this task) | ⬜ pending |
| 01-01-T2 | 01-01 | 1 | INFRA-02, INFRA-09 | unit | `cargo test -p onvif-server -- test_not_implemented_fault_has_ter_namespace test_token_constants_defined` | ✅ W0 (tests/foundation.rs created in 01-01 T2) | ⬜ pending |
| 01-02-T1 | 01-02 | 2 | INFRA-04 | build | `cargo build -p onvif-server` | ✅ W0 | ⬜ pending |
| 01-02-T2 | 01-02 | 2 | INFRA-03 | unit | `cargo test -p onvif-server -- test_embedded_wsdl_loader` | ✅ W0 (stub created in 01-01 T2) | ⬜ pending |
| 01-02-T3 | 01-02 | 2 | INFRA-05 | unit | `cargo test -p onvif-server -- test_not_implemented_returns_error` | ✅ W0 (stub created in 01-01 T2) | ⬜ pending |
| 01-03-T1 | 01-03 | 3 | INFRA-06, INFRA-07 | build | `cargo build -p onvif-server` | ✅ W0 | ⬜ pending |
| 01-03-T2 | 01-03 | 3 | INFRA-06, INFRA-08 | unit | `cargo test -p onvif-server -- test_builder_accepts_service_calls test_auth_bypass_includes_get_system_date_and_time` | ✅ W0 (stubs created in 01-01 T2) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 artifacts are created by plan 01-01, Task 1 and Task 2:

- [x] `Cargo.toml` — crate scaffold with soap-server path dependency (01-01 Task 1)
- [x] `src/lib.rs` — crate root with module declarations (01-01 Task 1)
- [x] `tests/foundation.rs` — test stubs for INFRA-02, INFRA-03, INFRA-05, INFRA-06, INFRA-08, INFRA-09 with `#[ignore]` markers so the file compiles before implementations exist (01-01 Task 2)

*All Wave 0 files are created within Phase 1 itself (greenfield crate). The stubs compile immediately; implementations remove the `#[ignore]` markers in later plans.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify — every task in all three plans has a concrete `cargo` command
- [x] Sampling continuity: no 3 consecutive tasks without automated verify — each task has its own verify command
- [x] Wave 0 covers all MISSING references — `tests/foundation.rs` stubs are created in 01-01 Task 2 before any plan 02/03 task references them
- [x] No watch-mode flags — all commands are one-shot (`cargo build`, `cargo test`)
- [x] Feedback latency < 15s — `cargo build` ~3s, `cargo test` ~10s on warm cache
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete
