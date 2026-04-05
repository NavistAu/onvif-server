---
phase: 3
slug: media-service
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-05
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | tokio::test + std::test |
| **Config file** | none — `[dev-dependencies]` in Cargo.toml |
| **Quick run command** | `cargo test -p onvif-server media` |
| **Full suite command** | `cargo test -p onvif-server` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p onvif-server`
- **After every plan wave:** Run `cargo test -p onvif-server`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 20 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-T1 | 03-01 | 1 | MEDIA-01 | unit | `cargo test -p onvif-server media_get_profiles` | ✅ W0 (created in first task) | ⬜ pending |
| 03-01-T2 | 03-01 | 1 | MEDIA-02, MEDIA-06 | unit | `cargo test -p onvif-server media_get_stream_uri media_get_snapshot_uri` | ✅ W0 | ⬜ pending |
| 03-01-T3 | 03-01 | 1 | MEDIA-03, MEDIA-04, MEDIA-05 | unit | `cargo test -p onvif-server media_get_video_sources media_get_video_source_configurations media_get_video_encoder_configurations` | ✅ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 artifacts are created within the first plan's initial task:

- [x] `tests/media_service.rs` — test stubs for MEDIA-01 through MEDIA-06 with `#[ignore]`
- [x] `VIDEO_ENCODER_TOKEN` constant added to `src/constants.rs`
- [x] Media types expanded in `src/generated/types.rs`

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify — every task has a concrete `cargo test` command
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references — test stubs and constants created in first task
- [x] No watch-mode flags — all commands are one-shot
- [x] Feedback latency < 20s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete
