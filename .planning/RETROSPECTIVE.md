# Project Retrospective

## Milestone: v1.0 — onvif-server

**Shipped:** 2026-04-05
**Phases:** 6 | **Plans:** 12

### What Was Built
- Spec-compliant ONVIF device server crate with 5 service traits and builder API
- DeviceServiceHandler with 7 Device Management operations + WS-Security auth
- MediaServiceHandler with 6 Profile S operations + Frigate PTZConfiguration
- PTZServiceHandler with 15 PTZ operations + TranslationSpaceFov + MoveStatus
- ImagingServiceHandler + EventServiceHandler with in-memory subscription state
- WS-Discovery UDP multicast (feature-gated) with configurable advertised host
- Frigate autotracker compat test + ODM smoke test + virtual_ptz example
- 56 tests, 0 failures, 42 requirements satisfied

### What Worked
- Porting approach: treating this as a port from prior art (onvif-rs, python-onvif-zeep, ONVIF spec) gave Claude full discretion on all decisions, eliminating discussion overhead
- Consistent handler pattern: extract_local_name + match dispatch replicated identically across all 5 services — each new service was faster to build than the last
- Research-driven planning: phase researchers identified Frigate-specific pitfalls (TranslationSpaceFov URI, MoveStatus attribute, token consistency) before any code was written
- TDD scaffold pattern: creating test stubs with #[ignore] in Wave 0, then implementing and enabling — caught regressions early
- Plan checker caught real issues: INFRA-04 empty type generation gap in Phase 1 would have broken Phase 2

### What Was Inefficient
- Hand-written types due to yaserde/xsd-parser toolchain constraint — will need to revisit when Rust 1.86+ is available
- Phase 2 auth tests deferred too long — became a gap closure item instead of being addressed in-phase
- WSDL external import stubs discovered late (Phase 5 execution) — could have been caught in Phase 1 research
- Phase 3 ROADMAP.md showed "In Progress" even after completion — tracking inconsistency from agent updates

### Patterns Established
- One handler struct per ONVIF service implementing SoapHandler
- Router::merge() for multi-service wiring with cloned auth credentials per service
- Token constants defined centrally, referenced across all services
- advertised_host builder field for configurable XAddr construction
- Integration tests call handlers directly (no HTTP) for speed; HTTP tests reserved for auth validation

### Key Lessons
- Feature-gated code (WS-Discovery) should be tested with the feature enabled in CI — easy to forget
- WSDL files with external imports need all transitive dependencies stubbed at crate setup time
- Auth bypass is scoped per-service, not globally — only device service receives GetSystemDateAndTime bypass

## Cross-Milestone Trends

| Metric | v1.0 |
|--------|------|
| Phases | 6 |
| Plans | 12 |
| Tests | 56 |
| Requirements | 42 |
| Gap closure phases | 1 |
| Audit iterations | 2 (first found gaps, second passed) |
