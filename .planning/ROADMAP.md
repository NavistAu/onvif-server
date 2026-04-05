# Roadmap: onvif-server

## Overview

Build a spec-compliant ONVIF device server crate in Rust. The journey starts with shared infrastructure (error types, WSDL loader, type strategy, token constants), then builds up the service stack one layer at a time: Device Management first to validate end-to-end wiring, Media next to establish profile tokens, PTZ as the primary deliverable with Frigate autotracker compatibility, and finally Imaging/Events/Discovery to round out the service surface and add the virtual_ptz example.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - Crate scaffold, error types, WSDL loader, ONVIF types, token constants, builder skeleton (completed 2026-04-05)
- [x] **Phase 2: Device Management** - Working ONVIF device endpoint with auth, GetSystemDateAndTime, GetCapabilities, GetServices, and device identity operations (completed 2026-04-05)
- [ ] **Phase 3: Media Service** - Profile S streaming metadata with correct PTZ profile structure for Frigate compatibility
- [x] **Phase 4: PTZ Service** - Full PTZ control surface with TranslationSpaceFov, MoveStatus, and Frigate end-to-end test (completed 2026-04-05)
- [ ] **Phase 5: Imaging, Events, Discovery, and Polish** - GetImagingSettings, Events service, WS-Discovery, virtual_ptz example, ODM smoke test
- [ ] **Phase 6: Gap Closure & Polish** - Advertised host for XAddrs, HTTP-level auth tests, PTZ constructor consistency

## Phase Details

### Phase 1: Foundation
**Goal**: The crate compiles with correct infrastructure for all downstream service phases — error types, WSDL loader, ONVIF type definitions, token constants, and builder skeleton in place
**Depends on**: Nothing (first phase)
**Requirements**: INFRA-01, INFRA-02, INFRA-03, INFRA-04, INFRA-05, INFRA-06, INFRA-07, INFRA-08, INFRA-09
**Success Criteria** (what must be TRUE):
  1. `cargo build` succeeds with no warnings on a fresh checkout
  2. `OnvifError::NotImplemented` serializes to a SOAP fault envelope that includes `xmlns:ter="http://www.onvif.org/ver10/error"` — verifiable by printing the fault XML
  3. `EmbeddedWsdlLoader` can return WSDL bytes for the Device, Media, and PTZ service WSDLs by name — verifiable with a unit test
  4. All token constants (profile, video source, PTZ node, PTZ config) are defined as crate-level `pub const` strings and used in at least one test
  5. `OnvifServer::builder()` compiles and accepts service registration calls even with no services registered yet
**Plans**: 3 plans

Plans:
- [ ] 01-01-PLAN.md — Crate scaffold, OnvifError with ter: namespace, token constants
- [ ] 01-02-PLAN.md — yaserde spike, WSDL file bundling, EmbeddedWsdlLoader, service trait stubs
- [ ] 01-03-PLAN.md — OnvifServer builder skeleton, auth wiring, full test suite green

### Phase 2: Device Management
**Goal**: A running ONVIF device endpoint answers all standard Device Management calls with correct responses — any ONVIF client can authenticate and retrieve device identity
**Depends on**: Phase 1
**Requirements**: DEV-01, DEV-02, DEV-03, DEV-04, DEV-05, DEV-06, DEV-07
**Success Criteria** (what must be TRUE):
  1. `GetSystemDateAndTime` returns HTTP 200 with current UTC time when called with no `Security` header
  2. `GetCapabilities` and `GetServices` both return service XAddrs that match the server's bound address
  3. `GetDeviceInformation` returns consumer-configured manufacturer, model, firmware version, serial number, and hardware ID
  4. An ONVIF client sending a valid WS-Security UsernameToken digest receives HTTP 200 on authenticated operations; an invalid credential receives a SOAP auth fault
**Plans**: 2 plans

Plans:
- [ ] 02-01-PLAN.md — Expanded types + DeviceService trait signatures + DeviceServiceHandler (GetSystemDateAndTime, GetCapabilities, GetServices, GetDeviceInformation)
- [ ] 02-02-PLAN.md — GetScopes, GetHostname, GetNetworkInterfaces handlers + OnvifServer::run() + auth wiring

### Phase 3: Media Service
**Goal**: A consumer can configure stream URIs and snapshot URIs, and any ONVIF client can retrieve complete Profile S media metadata including profiles with correct PTZ configuration references
**Depends on**: Phase 2
**Requirements**: MEDIA-01, MEDIA-02, MEDIA-03, MEDIA-04, MEDIA-05, MEDIA-06
**Success Criteria** (what must be TRUE):
  1. `GetProfiles` returns at least one profile whose `PTZConfiguration.DefaultContinuousPanTiltVelocitySpace` is set to the `TranslationSpaceFov` URI constant
  2. `GetStreamUri` returns the RTSP URL provided by the consumer's trait implementation
  3. `GetVideoSources`, `GetVideoSourceConfigurations`, and `GetVideoEncoderConfigurations` all return responses where token values match the crate-level token constants
  4. `GetSnapshotUri` returns the snapshot URL provided by the consumer's trait implementation
**Plans**: 2 plans

Plans:
- [ ] 03-01-PLAN.md — VIDEO_ENCODER_TOKEN, media type stubs, typed MediaService trait, MediaServiceHandler (all 6 operations), test scaffolding
- [ ] 03-02-PLAN.md — Wire MediaServiceHandler into run() via Router::merge(), crate-root exports, full test suite green

### Phase 4: PTZ Service
**Goal**: Frigate's autotracker runs successfully against the server — PTZ discovery, movement, status polling, and preset operations all work with correct coordinate spaces and capability advertisements
**Depends on**: Phase 3
**Requirements**: PTZ-01, PTZ-02, PTZ-03, PTZ-04, PTZ-05, PTZ-06, PTZ-07, PTZ-08, PTZ-09, PTZ-10, PTZ-11, PTZ-12, PTZ-13, PTZ-14, PTZ-15, TEST-01, TEST-02
**Success Criteria** (what must be TRUE):
  1. `GetNodes` response includes `TranslationSpaceFov` in `RelativePanTiltTranslationSpace` with the exact URI `http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov`
  2. `GetServiceCapabilities` response includes `MoveStatus="true"` — verifiable by the Frigate compat test asserting the attribute is present
  3. The Frigate autotracker call sequence (GetProfiles → GetConfigurationOptions → GetServiceCapabilities → GetStatus → RelativeMove → GotoPreset) runs end-to-end in `tests/frigate_compat.rs` without errors
  4. `GetStatus` returns `MoveStatus` with `PanTilt` and `Zoom` fields set to IDLE or MOVING based on the consumer's trait implementation
  5. The `virtual_ptz` example compiles and starts an ONVIF server with all PTZ operations implemented via a simple in-memory stub
**Plans**: 2 plans

Plans:
- [ ] 04-01-PLAN.md — PTZ types, typed PTZService trait, PTZServiceHandler with all 15 operations, unit test suite
- [ ] 04-02-PLAN.md — Wire PTZServiceHandler into run(), Frigate compat integration test, virtual_ptz example

### Phase 5: Imaging, Events, Discovery, and Polish
**Goal**: The full v1 service surface is complete — Imaging and Events trait APIs exist, WS-Discovery responds on multicast, and an ODM smoke test confirms basic discovery and info retrieval
**Depends on**: Phase 4
**Requirements**: IMG-01, EVT-01, EVT-02, EVT-03, EVT-04, DISC-01, DISC-02, TEST-03
**Success Criteria** (what must be TRUE):
  1. `GetImagingSettings` invokes the consumer's trait method and returns its result
  2. `CreatePullPointSubscription`, `PullMessages`, and `Unsubscribe` complete a full event subscription lifecycle without panics
  3. When the `discovery` feature flag is enabled, the server responds to a WS-Discovery Probe message sent to UDP 239.255.255.250:3702 with a ProbeMatch containing the server's XAddrs
  4. ONVIF Device Manager (ODM) can connect, list device information, and list services without errors — verified by TEST-03 smoke test steps
**Plans**: 2 plans

Plans:
- [ ] 05-01-PLAN.md — ImagingServiceHandler (IMG-01), EventServiceHandler with subscription state (EVT-01..04), typed trait signatures, unit test suites
- [ ] 05-02-PLAN.md — Wire Imaging/Events into run(), update DeviceServiceHandler XAddrs, WS-Discovery UDP task (DISC-01, DISC-02), virtual_ptz example extension, ODM smoke test (TEST-03)

### Phase 6: Gap Closure & Polish
**Goal**: Close all audit gaps — advertised host for real client connectivity, HTTP-level auth tests, and constructor consistency
**Depends on**: Phase 5
**Requirements**: DEV-02, DEV-03, DEV-05, DISC-01, INFRA-07, PTZ-01
**Gap Closure:** Closes gaps from v1.0 milestone audit
**Success Criteria** (what must be TRUE):
  1. `OnvifServerBuilder::advertised_host()` sets the host used in all XAddr construction — GetCapabilities, GetServices, and WS-Discovery ProbeMatch all return the configured host instead of `0.0.0.0`
  2. An HTTP-level test sends valid WS-Security credentials and receives HTTP 200; another sends invalid credentials and receives a SOAP auth fault
  3. `PTZServiceHandler` is constructed via `::new()` in server.rs, consistent with all other handlers
**Plans**: 1 plan

Plans:
- [ ] 06-01-PLAN.md — advertised_host builder field, HTTP-level auth tests, PTZServiceHandler::new() fix

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 3/3 | Complete   | 2026-04-05 |
| 2. Device Management | 2/2 | Complete   | 2026-04-05 |
| 3. Media Service | 2/2 | Complete   | 2026-04-05 |
| 4. PTZ Service | 2/2 | Complete   | 2026-04-05 |
| 5. Imaging, Events, Discovery, and Polish | 2/2 | Complete   | 2026-04-05 |
| 6. Gap Closure & Polish | 0/1 | Not started | - |
