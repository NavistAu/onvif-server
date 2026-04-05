# onvif-server

## What This Is

A Rust crate for building ONVIF-compliant device servers. Provides async trait-based APIs for each ONVIF service (Device Management, Media, PTZ, Imaging, Events, etc.), bundles official WSDLs/XSDs, and handles all ONVIF-specific protocol details. Built on the `soap-server` sibling crate. Designed as a general-purpose library anyone can use to expose ONVIF services from Rust.

## Core Value

A spec-compliant ONVIF device server that "just works" with real ONVIF clients — consumers implement trait methods for their hardware/logic, and the crate handles everything else (SOAP envelope, WS-Security, WSDL serving, type serialization, auth exemptions).

## Requirements

### Validated

- [x] Trait-based API for ONVIF services (DeviceService, MediaService, PTZService, ImagingService, EventService) — v1.0
- [x] Builder pattern for server construction with service registration — v1.0
- [x] Bundled official ONVIF WSDLs and XSDs — v1.0
- [x] ONVIF type definitions for all request/response structures (hand-written stubs; XSD codegen deferred to v2) — v1.0
- [x] WS-Security authentication with configurable credentials — v1.0
- [x] Auth exemption for GetSystemDateAndTime per ONVIF spec — v1.0
- [x] Default `not_implemented()` SOAP fault for unimplemented trait methods — v1.0
- [x] Frigate autotracker compatibility (RelativeMove, GetStatus, GetProfiles, GetConfigurationOptions, GetServiceCapabilities, presets) — v1.0
- [x] TranslationSpaceFov advertisement in PTZ node/configuration — v1.0
- [x] MoveStatus capability advertisement and IDLE/MOVING status polling — v1.0
- [x] WS-Discovery (feature-gated) for network auto-discovery — v1.0
- [x] ONVIF spec compliance across all implemented services — v1.0
- [x] Configurable advertised host for real-client connectivity — v1.0

### Active

(None yet — define v2 requirements with `/gsd:new-milestone`)

### Out of Scope

- Fovealink application logic — that's the downstream consumer, not this crate
- Camera hardware integration — this crate provides the server framework, not driver code
- ONVIF client functionality — this is server-side only

## Context

- Dependency chain: `soap-server` <- `onvif-server` <- `fovealink`
- `soap-server` is a sibling project at `~/ws/soap-server`, currently at phase 1 completion — provides SOAP transport, WSDL handling, and WS-Security foundation
- Primary validation target is Frigate's autotracker (python-onvif-zeep), but the crate must be spec-compliant for any ONVIF client
- This is essentially a port informed by examining the best prior art: onvif-rs (lumeohq), python-onvif-zeep, and the official ONVIF specification
- Part of the Fovealink project — an ONVIF PTZ proxy for Reolink cameras
- Design document at `docs/DESIGN.md` is a starting point, not a final spec — technical decisions (type generation strategy, exact trait signatures, etc.) to be resolved through research

## Constraints

- **Dependency**: Must build on `soap-server` crate (path dependency at `~/ws/soap-server`)
- **Spec compliance**: Must conform to ONVIF Device/Media/PTZ/Imaging/Events specifications
- **Ecosystem**: Standard Rust ecosystem conventions — MIT OR Apache-2.0 dual license, async/tokio, yaserde for XML
- **Compatibility**: Must work with Frigate's python-onvif-zeep client as first validation target

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Build on soap-server crate | Separation of concerns — SOAP transport vs ONVIF protocol layer | ✓ Good — clean layering, soap-server handles all SOAP/WS-Security concerns |
| Hand-written type stubs (Option B) | onvif-rs and xsd-parser both blocked by Rust 1.86/icu_* requirement; crate pinned to 1.85.1 | ✓ Good — unblocked Phase 1; codegen can be added in v2 when toolchain updates |
| Trait-based service API | Consumers implement what they support, unimplemented ops return SOAP faults | ✓ Good — 5 traits with not_implemented() defaults, proven by virtual_ptz example |
| WS-Discovery behind feature flag | Not needed for Frigate (direct URL), useful for NVR auto-discovery | ✓ Good — socket2 UDP multicast works; no overhead when feature disabled |
| advertised_host builder field | 0.0.0.0 XAddrs break real ONVIF clients; needed configurable host | ✓ Good — added in gap closure, defaults to 0.0.0.0 for backward compat |
| Per-service SoapHandler dispatch | One handler per service using extract_local_name + match | ✓ Good — consistent pattern across all 5 services, easy to extend |

## Current State

Shipped v1.0 with ~2,500 LOC Rust across 24 source files + 15 WSDL/XSD assets.
Tech stack: soap-server (path dep), axum 0.8, tokio 1, quick-xml 0.39, async-trait, bytes.
56 tests passing (unit + integration + Frigate compat + ODM smoke).
virtual_ptz example demonstrates full consumer API.

---
*Last updated: 2026-04-06 after v1.0 milestone*
