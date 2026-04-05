---
phase: 01-foundation
plan: 02
subsystem: infra
tags: [rust, cargo, onvif, wsdl, xsd, async-trait, serde, embedded-bytes]

# Dependency graph
requires:
  - phase: 01-foundation/01-01
    provides: OnvifError, not_implemented(), module skeleton, crate scaffold

provides:
  - Type strategy decision: Option B (hand-written stubs — onvif-schema and xsd-parser both require Rust 1.86)
  - DeviceInfo stub type in src/generated/types.rs — concrete ONVIF type for Phase 2
  - EmbeddedWsdlLoader implementing WsdlLoader via include_bytes! for all 7 ONVIF files
  - wsdl/ directory with official ONVIF WSDL + XSD files embedded at compile time
  - Five service traits (DeviceService, MediaService, PTZService, ImagingService, EventService)
  - All traits object-safe: Arc<dyn DeviceService> compiles in server.rs
  - WsdlLoader and WsdlError re-exported from crate root
affects: [01-03, all Phase 2+ plans]

# Tech tracking
tech-stack:
  added: [serde 1.x with derive, async-trait (already present, now actively used)]
  patterns:
    - EmbeddedWsdlLoader strips path prefix via rsplit('/') to handle relative schemaLocation imports
    - Five service traits all follow same pattern: #[async_trait], Send + Sync + 'static, all methods default to not_implemented()
    - Type stubs live in src/generated/types.rs — hand-written for Phase 1, XSD codegen deferred to Phase 2+
    - DeviceInfo exposed from crate root as concrete anchor type for downstream phases

key-files:
  created:
    - wsdl/devicemgmt.wsdl
    - wsdl/media.wsdl
    - wsdl/ptz.wsdl
    - wsdl/imaging.wsdl
    - wsdl/events.wsdl
    - wsdl/onvif.xsd
    - wsdl/common.xsd
    - src/wsdl_loader.rs
    - src/traits/device.rs
    - src/traits/media.rs
    - src/traits/ptz.rs
    - src/traits/imaging.rs
    - src/traits/events.rs
    - src/generated/mod.rs
    - src/generated/types.rs
    - build.rs
  modified:
    - Cargo.toml
    - src/lib.rs
    - src/traits/mod.rs
    - src/server.rs
    - tests/foundation.rs

key-decisions:
  - "Type strategy Option B (hand-written stubs): onvif-schema git dep requires Rust 1.86 via icu_* transitive deps; xsd-parser 1.5 also requires 1.86 via url->idna->idna_adapter->icu_normalizer chain; crate is pinned to 1.85.1 — both codegen approaches blocked until toolchain upgrades"
  - "DeviceInfo is a Phase 1 hand-written stub with 5 fields — full XSD-derived codegen deferred to Phase 2+ when toolchain constraint resolves"
  - "EmbeddedWsdlLoader uses rsplit('/').next() to extract filename from relative schemaLocation paths like ../../../ver10/schema/onvif.xsd"
  - "events.wsdl stored locally under that name but the ONVIF source URL and import references use event.wsdl — both matched in loader"

patterns-established:
  - "Pattern 4: All service traits use #[async_trait] + Send + Sync + 'static + not_implemented() defaults — implementors override only what they need"
  - "Pattern 5: Generated types live in src/generated/ — hand-written stubs in Phase 1, XSD codegen drops in same location in Phase 2+"
  - "Pattern 6: WSDLs bundled via include_bytes! in EmbeddedWsdlLoader — path stripping handles relative import locations from official ONVIF WSDLs"

requirements-completed: [INFRA-03, INFRA-04, INFRA-05]

# Metrics
duration: 6min
completed: 2026-04-05
---

# Phase 1 Plan 02: WSDL Bundling and Service Traits Summary

**Seven official ONVIF WSDL/XSD files embedded at compile time, EmbeddedWsdlLoader resolving relative imports, five async service traits with not_implemented() defaults, and DeviceInfo stub type — all compiling on Rust 1.85.1 via hand-written Option B stubs**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-04-05T07:22:12Z
- **Completed:** 2026-04-05T07:28:24Z
- **Tasks:** 3
- **Files modified:** 16 created, 5 modified

## Accomplishments

- `EmbeddedWsdlLoader::load("devicemgmt.wsdl")` returns `Ok(bytes)` with 200 KB of content (test passes)
- All five service traits compile as dyn-dispatchable objects — `Arc<dyn DeviceService>` verified in server.rs
- `test_not_implemented_returns_error` passes: `StubDevice` with zero overrides returns `Err(OnvifError::NotImplemented)` from default method
- `DeviceInfo` struct (manufacturer, model, firmware_version, serial_number, hardware_id) accessible as `onvif_server::DeviceInfo`
- `WsdlLoader` re-exported from crate root — test code imports `onvif_server::WsdlLoader` without direct soap-server dep

## Task Commits

Each task was committed atomically:

1. **Task 1: Type strategy spike and Cargo.toml update** - `a708181` (feat)
2. **Task 2: WSDL downloads and EmbeddedWsdlLoader** - `111d356` (feat)
3. **Task 3: Service trait stubs (TDD RED)** - `d2097a7` (test)
4. **Task 3: Service trait stubs (TDD GREEN)** - `c4e355f` (feat)

## Files Created/Modified

- `/Users/jhogendorn/ws/onvif-server/wsdl/devicemgmt.wsdl` - Official ONVIF Device Management WSDL (200 KB)
- `/Users/jhogendorn/ws/onvif-server/wsdl/media.wsdl` - Official ONVIF Media WSDL (175 KB)
- `/Users/jhogendorn/ws/onvif-server/wsdl/ptz.wsdl` - Official ONVIF PTZ WSDL (32 KB)
- `/Users/jhogendorn/ws/onvif-server/wsdl/imaging.wsdl` - Official ONVIF Imaging WSDL (27 KB)
- `/Users/jhogendorn/ws/onvif-server/wsdl/events.wsdl` - Official ONVIF Events WSDL (48 KB)
- `/Users/jhogendorn/ws/onvif-server/wsdl/onvif.xsd` - Official ONVIF XSD schema (418 KB)
- `/Users/jhogendorn/ws/onvif-server/wsdl/common.xsd` - Official ONVIF common XSD (19 KB)
- `/Users/jhogendorn/ws/onvif-server/src/wsdl_loader.rs` - EmbeddedWsdlLoader with include_bytes! and path stripping
- `/Users/jhogendorn/ws/onvif-server/src/traits/device.rs` - DeviceService (9 methods)
- `/Users/jhogendorn/ws/onvif-server/src/traits/media.rs` - MediaService (6 methods)
- `/Users/jhogendorn/ws/onvif-server/src/traits/ptz.rs` - PTZService (11 methods)
- `/Users/jhogendorn/ws/onvif-server/src/traits/imaging.rs` - ImagingService (6 methods)
- `/Users/jhogendorn/ws/onvif-server/src/traits/events.rs` - EventService (6 methods)
- `/Users/jhogendorn/ws/onvif-server/src/generated/types.rs` - DeviceInfo hand-written stub
- `/Users/jhogendorn/ws/onvif-server/src/generated/mod.rs` - Generated module root
- `/Users/jhogendorn/ws/onvif-server/build.rs` - Build script stub (no codegen in Phase 1)
- `/Users/jhogendorn/ws/onvif-server/Cargo.toml` - Added serde dep, removed xsd-parser build-dep
- `/Users/jhogendorn/ws/onvif-server/src/lib.rs` - Added WsdlLoader, WsdlError, EmbeddedWsdlLoader, DeviceInfo, five trait re-exports
- `/Users/jhogendorn/ws/onvif-server/src/traits/mod.rs` - Now re-exports all five service traits
- `/Users/jhogendorn/ws/onvif-server/src/server.rs` - Added Arc<dyn DeviceService> field for dyn-safety validation
- `/Users/jhogendorn/ws/onvif-server/tests/foundation.rs` - Implemented test_embedded_wsdl_loader and test_not_implemented_returns_error

## Decisions Made

- **Option B (hand-written stubs) over both codegen options:** Option A (onvif-schema git dep) failed — transitive icu_* dependencies (pulled via the onvif-rs workspace) require Rust 1.86. Option B as specified (xsd-parser 1.5 build-dep) also fails — xsd-parser pulls in `url` → `idna` → `idna_adapter` → `icu_normalizer` chain requiring Rust 1.86. With both blocked, Phase 1 uses hand-written stubs; XSD codegen is deferred to Phase 2+ when either (a) the toolchain pin is raised to 1.86+ or (b) compatible versions of these libraries become available.

- **xsd-parser removed from build-dependencies:** Including it as a build-dep with zero usage still pulls in the icu_* chain and breaks the build on 1.85.1. Removed entirely for Phase 1; add back in Phase 2 when toolchain allows.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Option A (onvif-schema) rejected due to Rust version conflict**
- **Found during:** Task 1 (type strategy spike)
- **Issue:** `onvif-schema` git dep brought in `icu_collections`, `icu_normalizer`, etc. all requiring rustc 1.86; crate pinned to 1.85.1
- **Fix:** Removed onvif-schema dep, proceeded to Option B
- **Verification:** Removed dep, re-attempted build

**2. [Rule 3 - Blocking] xsd-parser build-dep also requires Rust 1.86 — removed**
- **Found during:** Task 1 (Option B attempt)
- **Issue:** xsd-parser 1.5 pulls in `url` crate → `idna 1.1` → `idna_adapter 1.2` → `icu_normalizer 2.2.0` (requires 1.86). Build fails identically to Option A.
- **Fix:** Removed `[build-dependencies]` section entirely. DeviceInfo stub is hand-written; build.rs is a no-op. This is the pragmatic Phase 1 approach — the plan itself says "Full XSD-derived codegen is a Phase 2+ concern."
- **Files modified:** Cargo.toml, build.rs
- **Verification:** `cargo build -p onvif-server` succeeds
- **Committed in:** a708181 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both blocking — same root cause: icu_* Rust version gate)
**Impact on plan:** The INFRA-04 requirement (concrete ONVIF type accessible from crate) is fully satisfied by the hand-written DeviceInfo stub. The only deferral is automated XSD codegen, which was already labeled as a Phase 2+ concern in the plan itself. No functional scope is lost.

## Issues Encountered

Both codegen approaches (onvif-schema and xsd-parser) are gated by the same icu_* dependency chain requiring Rust 1.86. The workaround (hand-written stubs) is explicitly sanctioned by the plan. Phase 2 plans should note: to enable XSD codegen, either upgrade rust-toolchain.toml to 1.86+ or pin `idna` to an older version before `idna_adapter` introduced the icu dependency.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 03 (server builder) can begin immediately
- All five service traits exported from crate root and ready for use in OnvifServerBuilder
- `Arc<dyn DeviceService>` confirmed dyn-safe
- `DeviceInfo` accessible as `onvif_server::DeviceInfo` for Phase 2 type usage
- The two plan-03 test stubs (`test_builder_accepts_service_calls`, `test_auth_bypass_includes_get_system_date_and_time`) remain `#[ignore]` pending plan 03

---
*Phase: 01-foundation*
*Completed: 2026-04-05*

## Self-Check: PASSED

- FOUND: src/wsdl_loader.rs
- FOUND: wsdl/devicemgmt.wsdl
- FOUND: src/traits/device.rs
- FOUND: src/generated/types.rs
- FOUND: .planning/phases/01-foundation/01-02-SUMMARY.md
- FOUND: a708181 (Task 1 commit)
- FOUND: 111d356 (Task 2 commit)
- FOUND: d2097a7 (Task 3 RED commit)
- FOUND: c4e355f (Task 3 GREEN commit)
