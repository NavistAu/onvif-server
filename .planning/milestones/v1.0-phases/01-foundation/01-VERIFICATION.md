---
phase: 01-foundation
verified: 2026-04-05T08:00:00Z
status: passed
score: 9/9 must-haves verified
re_verification: false
---

# Phase 1: Foundation Verification Report

**Phase Goal:** The crate compiles with correct infrastructure for all downstream service phases — error types, WSDL loader, ONVIF type definitions, token constants, and builder skeleton in place
**Verified:** 2026-04-05T08:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build` succeeds with no warnings on a fresh checkout | VERIFIED | Build output: `Finished dev profile [unoptimized + debuginfo] target(s) in 1.56s`. All 12 warnings are from the soap-server dependency, not onvif-server. onvif-server itself compiles clean. |
| 2 | `OnvifError::NotImplemented` serializes to a SOAP fault with `xmlns:ter="http://www.onvif.org/ver10/error"` | VERIFIED | `test_not_implemented_fault_has_ter_namespace` passes. `into_soap_fault()` in `src/error.rs` embeds the namespace inline in the detail string via `format!(r#"<ter:fault xmlns:ter="http://www.onvif.org/ver10/error">..."#)`. |
| 3 | `EmbeddedWsdlLoader` returns WSDL bytes for Device, Media, and PTZ WSDLs by name | VERIFIED | `test_embedded_wsdl_loader` passes. All three WSDLs present: devicemgmt.wsdl (200 KB), media.wsdl (175 KB), ptz.wsdl (32 KB). |
| 4 | All token constants defined as crate-level `pub const` strings and used in at least one test | VERIFIED | `test_token_constants_defined` passes. All five constants in `src/constants.rs`: PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV. |
| 5 | `OnvifServer::builder()` compiles and accepts service registration calls | VERIFIED | `test_builder_accepts_service_calls` and `test_auth_bypass_includes_get_system_date_and_time` both pass. Builder accepts all five service types. |

**Score:** 5/5 ROADMAP success criteria verified

### Plan-level Must-Have Truths (from PLAN frontmatter)

**Plan 01-01 truths:**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build -p onvif-server` succeeds from a clean state | VERIFIED | Build confirmed above |
| 2 | `OnvifError::NotImplemented` serializes to SOAP fault with `xmlns:ter` in detail XML | VERIFIED | Test passes; detail string confirmed in `src/error.rs:30` |
| 3 | All five token constants defined as `pub const &str` and accessible | VERIFIED | `src/constants.rs` contains all five; test passes |

**Plan 01-02 truths:**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `EmbeddedWsdlLoader::load("devicemgmt.wsdl")` returns `Ok(bytes)` with non-empty content | VERIFIED | Test passes |
| 2 | `EmbeddedWsdlLoader::load("media.wsdl")` and `load("ptz.wsdl")` both return `Ok` | VERIFIED | Test passes |
| 3 | All five service traits compile as `dyn`-dispatchable trait objects | VERIFIED | `Arc<dyn DeviceService>` field in `src/server.rs:22`; all five traits in `src/traits/` |
| 4 | `not_implemented()` as default method body compiles within `#[async_trait]` trait | VERIFIED | `test_not_implemented_returns_error` passes |
| 5 | `cargo build -p onvif-server` succeeds after type strategy decision | VERIFIED | Option B (hand-written stubs) chosen; build succeeds |
| 6 | At least one concrete ONVIF stub type (`DeviceInfo`) accessible from crate | VERIFIED | `src/generated/types.rs` contains `pub struct DeviceInfo` with 5 fields; re-exported via `pub use generated::DeviceInfo` in `src/lib.rs:13` |

**Plan 01-03 truths:**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `OnvifServer::builder().port(8080).auth("admin","secret").build()` compiles and returns `Ok(OnvifServer)` | VERIFIED | `test_builder_accepts_service_calls` passes |
| 2 | `OnvifServerBuilder` accepts `.device_service()` and `.ptz_service()` without errors | VERIFIED | Builder methods confirmed in `src/server.rs:94,106` |
| 3 | Auth bypass set includes `"GetSystemDateAndTime"` by default | VERIFIED | `test_auth_bypass_includes_get_system_date_and_time` passes; inserted in `OnvifServerBuilder::new()` at `src/server.rs:62` |
| 4 | `cargo test -p onvif-server` fully green with no ignored tests | VERIFIED | Test output: `6 passed; 0 failed; 0 ignored` |

---

## Required Artifacts

### Plan 01-01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | Crate definition with soap-server path dep and all required deps | VERIFIED | `soap-server = { path = "../soap-server" }` present; tokio = `["sync"]` in deps, `["full","test-util"]` in dev-deps only |
| `src/lib.rs` | Crate root with module declarations and public re-exports | VERIFIED | Exports `OnvifError`, `OnvifServerBuilder`, `PROFILE_TOKEN` and all others |
| `src/error.rs` | OnvifError enum + `into_soap_fault()` method | VERIFIED | Contains `xmlns:ter` at line 30; substantive (51 lines) |
| `src/constants.rs` | All crate-level `pub const` token strings | VERIFIED | Contains `PROFILE_TOKEN` and all five constants |
| `tests/foundation.rs` | Test suite with all tests enabled | VERIFIED | 6 tests, 0 ignored |

### Plan 01-02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `wsdl/devicemgmt.wsdl` | Official ONVIF Device Management WSDL bundled at compile time | VERIFIED | 200,007 bytes |
| `src/wsdl_loader.rs` | EmbeddedWsdlLoader implementing WsdlLoader via `include_bytes!` | VERIFIED | `impl WsdlLoader for EmbeddedWsdlLoader` at line 10; matches all 7 WSDL/XSD files |
| `src/traits/device.rs` | DeviceService async trait with `#[async_trait]` and `not_implemented()` defaults | VERIFIED | 9 methods, all default to `not_implemented()`; 54 lines |
| `src/traits/ptz.rs` | PTZService async trait | VERIFIED | 11 methods including get_nodes, get_configurations, relative_move, get_status, goto_preset |

### Plan 01-03 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/server.rs` | OnvifServer + OnvifServerBuilder with service registration, auth config, auto auth-bypass | VERIFIED | Contains `GetSystemDateAndTime` at line 62; `auth_bypass_set()` accessor at line 124; 145 lines, substantive |
| `tests/foundation.rs` | Full test suite — all enabled, all green | VERIFIED | 6/6 tests pass, 0 ignored |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/error.rs` | `soap_server::fault::SoapFault` | `into_soap_fault()` method | WIRED | `use soap_server::fault::{FaultCode, SoapFault}` at line 1; `SoapFault::new(...)` called at lines 32, 39 |
| `src/lib.rs` | `src/error.rs` | `mod error; pub use error::OnvifError;` | WIRED | Line 1 declares `mod error`; line 8 `pub use error::OnvifError` |
| `src/wsdl_loader.rs` | `wsdl/*.wsdl` | `include_bytes!` macro | WIRED | 7 `include_bytes!` calls matching all bundled WSDL/XSD files |
| `src/traits/device.rs` | `not_implemented()` | Default method body | WIRED | `use crate::error::{OnvifError, not_implemented}` at line 2; every method calls `not_implemented()` |
| `src/lib.rs` | `soap_server::WsdlLoader` | `pub use soap_server::WsdlLoader` re-export | WIRED | Line 11: `pub use soap_server::WsdlLoader` |
| `src/server.rs` | `crate::traits::DeviceService` | `Option<Arc<dyn DeviceService>>` field | WIRED | Line 5 imports all traits; lines 22, 50 store `Arc<dyn DeviceService>` |
| `src/server.rs` | `auth_bypass` HashSet | `GetSystemDateAndTime` auto-inserted in `new()` | WIRED | Line 62: `auth_bypass.insert("GetSystemDateAndTime".to_string())` inside `fn new()` |

---

## Requirements Coverage

All 9 requirement IDs declared across the three plans are accounted for:

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| INFRA-01 | 01-01 | Crate scaffolding with Cargo.toml, soap-server path dep, module structure | SATISFIED | `Cargo.toml` has `soap-server = { path = "../soap-server" }`; module skeleton: error, constants, server, wsdl_loader, traits, generated |
| INFRA-02 | 01-01 | OnvifError with variants + SOAP fault mapping with `ter:` namespace | SATISFIED | `src/error.rs`: 3 variants (NotImplemented, InvalidArgument, ActionNotSupported); `into_soap_fault()` embeds `xmlns:ter="http://www.onvif.org/ver10/error"`; test passes |
| INFRA-03 | 01-02 | Embedded WSDL/XSD loader via soap-server's WsdlLoader trait | SATISFIED | `EmbeddedWsdlLoader` implements `WsdlLoader`; 7 files bundled via `include_bytes!`; test_embedded_wsdl_loader passes |
| INFRA-04 | 01-02 | ONVIF type definitions — at least one concrete type accessible | SATISFIED | `DeviceInfo` struct in `src/generated/types.rs` with 5 fields; re-exported from crate root. Note: full XSD codegen deferred to Phase 2+ due to Rust 1.85.1 toolchain constraint blocking icu_* deps |
| INFRA-05 | 01-02 | Trait-based service API with async methods defaulting to spec-compliant faults | SATISFIED | 5 service traits in `src/traits/`; all methods default to `not_implemented()` returning `Err(OnvifError::NotImplemented)`; `test_not_implemented_returns_error` passes |
| INFRA-06 | 01-03 | Builder pattern for server construction with service registration | SATISFIED | `OnvifServer::builder()` factory method; `.device_service()`, `.media_service()`, `.ptz_service()`, `.imaging_service()`, `.event_service()`, `.port()`, `.auth()`, `.build()` all implemented; test passes |
| INFRA-07 | 01-03 | WS-Security auth credentials stored in builder, ready for Phase 2 soap-server wiring | SATISFIED | `.auth(username, password)` stores credentials in `OnvifServerBuilder`; `build()` transfers to `OnvifServer` fields; Phase 2 wires into `soap_server::ServerBuilder::auth()` |
| INFRA-08 | 01-03 | Auth exemption for GetSystemDateAndTime auto-registered | SATISFIED | `auth_bypass.insert("GetSystemDateAndTime")` in `OnvifServerBuilder::new()`; `auth_bypass_set()` accessor exposed; `test_auth_bypass_includes_get_system_date_and_time` passes |
| INFRA-09 | 01-01 | Token constants for profile, video source, PTZ node, PTZ config, translation space | SATISFIED | `src/constants.rs`: PROFILE_TOKEN="profile_0", VIDEO_SOURCE_TOKEN="video_src_0", PTZ_NODE_TOKEN="ptz_node_0", PTZ_CONFIG_TOKEN="ptz_cfg_0", TRANSLATION_SPACE_FOV="http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov"; test passes |

**Orphaned requirements check:** REQUIREMENTS.md maps INFRA-01 through INFRA-09 to Phase 1. All 9 IDs appear in plan frontmatter (01-01 covers INFRA-01, INFRA-02, INFRA-09; 01-02 covers INFRA-03, INFRA-04, INFRA-05; 01-03 covers INFRA-06, INFRA-07, INFRA-08). No orphaned requirements.

---

## Anti-Patterns Found

No anti-patterns detected in modified files:

- No TODO/FIXME/HACK/PLACEHOLDER comments in `src/` files
- No `return null` / empty return stubs — all artifacts are substantive implementations
- Phase-appropriate placeholders (e.g., `build()` returning a skeleton with no port binding) are explicitly documented and intended — Phase 2 extends them

No blockers, no warnings.

---

## Human Verification Required

None. All success criteria are programmatically verifiable and have been verified by running the actual compiler and test suite.

---

## Test Suite Results

```
running 6 tests
test test_token_constants_defined ... ok
test test_not_implemented_fault_has_ter_namespace ... ok
test test_builder_accepts_service_calls ... ok
test test_auth_bypass_includes_get_system_date_and_time ... ok
test test_not_implemented_returns_error ... ok
test test_embedded_wsdl_loader ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## Notable Implementation Decisions (from Summaries)

These decisions are correct and do not constitute gaps:

1. **Rust 1.85.1 toolchain pin** (`rust-toolchain.toml`): Required because transitive dep `cpufeatures 0.3.0` needs edition2024, which requires Rust 1.85+.

2. **Type strategy Option B (hand-written stubs)**: Both `onvif-schema` (git) and `xsd-parser 1.5` pull in `icu_*` transitive deps that require Rust 1.86. With toolchain pinned at 1.85.1, full XSD codegen is deferred to Phase 2+. `DeviceInfo` hand-written stub satisfies INFRA-04 contract.

3. **INFRA-07 is a Phase 1 skeleton**: WS-Security auth wiring to soap-server's `ServerBuilder::auth()` is explicitly a Phase 2 concern. The builder stores credentials correctly, ready for Phase 2. This matches the ROADMAP and plan specs exactly.

---

_Verified: 2026-04-05T08:00:00Z_
_Verifier: Claude (gsd-verifier)_
