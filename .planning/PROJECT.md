# onvif-server

## What This Is

A Rust crate for building ONVIF-compliant device servers. Provides async trait-based APIs for each ONVIF service (Device Management, Media, PTZ, Imaging, Events, etc.), bundles official WSDLs/XSDs, and handles all ONVIF-specific protocol details. Built on the `soap-server` sibling crate. Designed as a general-purpose library anyone can use to expose ONVIF services from Rust.

## Core Value

A spec-compliant ONVIF device server that "just works" with real ONVIF clients — consumers implement trait methods for their hardware/logic, and the crate handles everything else (SOAP envelope, WS-Security, WSDL serving, type serialization, auth exemptions).

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Trait-based API for ONVIF services (DeviceService, MediaService, PTZService, ImagingService, EventService)
- [ ] Builder pattern for server construction with service registration
- [ ] Bundled official ONVIF WSDLs and XSDs
- [ ] ONVIF type definitions for all request/response structures (via onvif-rs or generated)
- [ ] WS-Security authentication with configurable credentials
- [ ] Auth exemption for GetSystemDateAndTime per ONVIF spec
- [ ] Default `not_implemented()` SOAP fault for unimplemented trait methods
- [ ] Frigate autotracker compatibility (RelativeMove, GetStatus, GetProfiles, GetConfigurationOptions, GetServiceCapabilities, presets)
- [ ] TranslationSpaceFov advertisement in PTZ node/configuration
- [ ] MoveStatus capability advertisement and IDLE/MOVING status polling
- [ ] WS-Discovery (feature-gated) for network auto-discovery
- [ ] ONVIF spec compliance across all implemented services

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
| Build on soap-server crate | Separation of concerns — SOAP transport vs ONVIF protocol layer | — Pending |
| Type definition strategy (onvif-rs vs generate) | Need to evaluate onvif-rs types for server-side usability | — Pending |
| Trait-based service API | Consumers implement what they support, unimplemented ops return SOAP faults | — Pending |
| WS-Discovery behind feature flag | Not needed for Frigate (direct URL), useful for NVR auto-discovery | — Pending |

---
*Last updated: 2026-04-05 after initialization*
