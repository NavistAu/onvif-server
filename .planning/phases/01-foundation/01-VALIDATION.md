---
phase: 1
slug: foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
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
| 01-01-01 | 01 | 1 | INFRA-01 | build | `cargo build -p onvif-server` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | INFRA-02 | unit | `cargo test -p onvif-server -- test_not_implemented_fault_has_ter_namespace` | ❌ W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | INFRA-03 | unit | `cargo test -p onvif-server -- test_embedded_wsdl_loader` | ❌ W0 | ⬜ pending |
| 01-01-04 | 01 | 1 | INFRA-04 | build | `cargo build -p onvif-server` | ❌ W0 | ⬜ pending |
| 01-01-05 | 01 | 1 | INFRA-05 | unit | `cargo test -p onvif-server -- test_not_implemented_returns_error` | ❌ W0 | ⬜ pending |
| 01-01-06 | 01 | 1 | INFRA-06 | unit | `cargo test -p onvif-server -- test_builder_accepts_service_calls` | ❌ W0 | ⬜ pending |
| 01-01-07 | 01 | 1 | INFRA-07 | build | `cargo build -p onvif-server` | ❌ W0 | ⬜ pending |
| 01-01-08 | 01 | 1 | INFRA-08 | unit | `cargo test -p onvif-server -- test_auth_bypass_includes_get_system_date_and_time` | ❌ W0 | ⬜ pending |
| 01-01-09 | 01 | 1 | INFRA-09 | unit | `cargo test -p onvif-server -- test_token_constants_defined` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml` — crate scaffold with soap-server path dependency, required before any Rust file compiles
- [ ] `src/lib.rs` — crate root with module declarations
- [ ] `tests/foundation.rs` — test stubs for INFRA-02, INFRA-03, INFRA-05, INFRA-06, INFRA-08, INFRA-09

*Existing infrastructure: none — this is a greenfield crate.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
