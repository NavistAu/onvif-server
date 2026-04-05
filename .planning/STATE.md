---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Completed 04-ptz-service/04-01-PLAN.md
last_updated: "2026-04-05T11:30:57.879Z"
last_activity: 2026-04-05 — MediaServiceHandler implemented; all 7 media service operations pass with green tests
progress:
  total_phases: 5
  completed_phases: 3
  total_plans: 9
  completed_plans: 8
  percent: 45
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-05)

**Core value:** A spec-compliant ONVIF device server that "just works" with real ONVIF clients — consumers implement trait methods, the crate handles everything else.
**Current focus:** Phase 2 — Device Management

## Current Position

Phase: 3 of 5 (Media Service)
Plan: 1 of 2 in current phase
Status: Executing — 03-01 complete, ready for 03-02
Last activity: 2026-04-05 — MediaServiceHandler implemented; all 7 media service operations pass with green tests

Progress: [#####░░░░░] 45%

## Performance Metrics

**Velocity:**
- Total plans completed: 4
- Average duration: ~18 min
- Total execution time: ~1.2 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-foundation | 3 | ~54 min | ~18 min |
| 02-device-management | 2 | ~26 min | ~13 min |

**Recent Trend:**
- Last 5 plans: -
- Trend: -

*Updated after each plan completion*
| Phase 01-foundation P01 | 15 | 2 tasks | 9 files |
| Phase 01-foundation P02 | 6 | 3 tasks | 21 files |
| Phase 01-foundation P03 | 2 | 2 tasks | 3 files |
| Phase 02-device-management P01 | 20 | 2 tasks | 10 files |
| Phase 02-device-management P02 | 6 | 2 tasks | 9 files |
| Phase 03-media-service P01 | 5 | 3 tasks | 7 files |
| Phase 03-media-service P02 | 5 | 2 tasks | 4 files |
| Phase 04-ptz-service P01 | 5 | 3 tasks | 7 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Phase 1: Type definition strategy (onvif-rs vs xsd-parser) — PENDING. Research flags a yaserde 0.7 vs 0.12 version conflict risk. A 30-minute compile spike is required before finalizing Phase 1 scope. If onvif-rs fails to compile cleanly, Option B (xsd-parser + build.rs) expands Phase 1 scope.
- Phase 1: `soap-server` WsdlLoader trait exact interface — must be confirmed from source before writing EmbeddedWsdlLoader.
- [Phase 01-foundation]: Pinned rust-toolchain.toml to 1.85.1 — cpufeatures 0.3.0 requires edition2024, unsupported by system Rust 1.79.0
- [Phase 01-foundation]: xmlns:ter namespace declared inline in SoapFault detail — soap-server envelope does not inject ONVIF namespaces
- [Phase 01-foundation]: Type strategy Option B (hand-written stubs): both onvif-schema and xsd-parser require Rust 1.86 via icu_* chain; crate pinned to 1.85.1 — DeviceInfo is a Phase 1 hand-written stub, XSD codegen deferred to Phase 2+
- [Phase 01-foundation]: build() returns Ok(OnvifServer) skeleton with no network activity — soap_server::ServerBuilder wiring deferred to Phase 2 per plan
- [Phase 01-foundation]: GetSystemDateAndTime inserted into auth_bypass at OnvifServerBuilder::new() — invariant holds even if builder is inspected before build()
- [Phase 02-device-management P01]: GetCapabilities and GetServices are handler-internal — not on DeviceService trait; handler builds XML from bound xaddr
- [Phase 02-device-management P01]: not_implemented() made generic (Result<T, OnvifError>) to serve typed trait stubs
- [Phase 02-device-management P01]: get_system_date_and_time defaults to Ok(Utc::now()) — always returns time without requiring implementor override
- [Phase 02-device-management P02]: axum added as direct dependency — needed for axum::serve in run() (was only transitive via soap-server)
- [Phase 02-device-management P02]: Auth tests (valid/invalid WS-Security credential) remain #[ignore] — digest construction deferred to ODM smoke test in Phase 5
- [Phase 02-device-management P02]: EmbeddedWsdlLoader extended with 4 W3C/OASIS XSD stubs — onvif.xsd imports require resolution at ServerBuilder::build time
- [Phase 03-media-service P01]: MediaService trait has only 2 methods (get_stream_uri, get_snapshot_uri); GetProfiles/GetVideoSources/etc. are handler-internal static responses from constants
- [Phase 03-media-service P01]: quick-xml 0.39 BytesText uses std::str::from_utf8(t.as_ref()) not t.unescape() for text content extraction
- [Phase 03-media-service P01]: GetProfiles PTZConfiguration always includes DefaultContinuousPanTiltVelocitySpace=TRANSLATION_SPACE_FOV for Frigate PTZ autotracking compatibility
- [Phase 03-media-service]: media_service required at run() time via ok_or(); std::iter::empty() for media auth_bypass; generated/mod.rs must re-export types before lib.rs
- [Phase 04-ptz-service]: PTZService trait has 9 control methods only; discovery operations are handler-internal static XML
- [Phase 04-ptz-service]: OnvifError has no NotFound variant — used InvalidArgument for unknown token errors in GetNode/GetConfiguration
- [Phase 04-ptz-service]: PTZServiceHandler has no xaddr field — PTZ service does not advertise a separate service URL

### Pending Todos

None yet.

### Blockers/Concerns

- **yaserde version compat:** onvif-rs pins yaserde 0.7; this crate targets 0.12. Must resolve before committing to Option A in Phase 1. See research/SUMMARY.md Gaps section.

## Session Continuity

Last session: 2026-04-05T11:30:57.876Z
Stopped at: Completed 04-ptz-service/04-01-PLAN.md
Resume file: None
