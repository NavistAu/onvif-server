# Requirements: onvif-server

**Defined:** 2026-04-05
**Core Value:** A spec-compliant ONVIF device server that "just works" with real ONVIF clients — consumers implement trait methods, the crate handles everything else.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Infrastructure

- [ ] **INFRA-01**: Crate scaffolding with Cargo.toml, soap-server path dependency, and module structure
- [ ] **INFRA-02**: OnvifError type with variants for NotImplemented, InvalidArgument, ActionNotSupported, and mapping to SOAP faults with ONVIF error namespace (`ter:`)
- [ ] **INFRA-03**: Embedded WSDL/XSD loader that serves bundled official ONVIF WSDLs and schemas via soap-server's WsdlLoader trait
- [ ] **INFRA-04**: ONVIF type definitions for all request/response structures (via onvif-rs schema crates or generated from bundled XSDs)
- [ ] **INFRA-05**: Trait-based service API where each ONVIF service is a Rust trait with async methods; unimplemented methods return spec-compliant SOAP faults by default
- [ ] **INFRA-06**: Builder pattern (`OnvifServer::builder()`) for server construction with service registration, auth config, and port binding
- [ ] **INFRA-07**: WS-Security UsernameToken digest authentication delegated to soap-server, with configurable credentials via builder
- [ ] **INFRA-08**: Auth exemption for GetSystemDateAndTime automatically registered (per ONVIF spec, accessible without authentication)
- [ ] **INFRA-09**: Token constants for consistent profile, video source, PTZ node, and PTZ configuration tokens across all services

### Device Management

- [ ] **DEV-01**: User can call GetSystemDateAndTime without authentication and receive current UTC time and timezone
- [ ] **DEV-02**: User can call GetCapabilities and receive XAddrs (service URLs) for all registered services
- [ ] **DEV-03**: User can call GetServices and receive service namespace, XAddr, and capabilities for all registered services
- [ ] **DEV-04**: User can call GetDeviceInformation and receive manufacturer, model, firmware version, serial number, and hardware ID as configured by the consumer
- [ ] **DEV-05**: User can call GetScopes and receive ONVIF-standard scope URIs identifying device type and name
- [ ] **DEV-06**: User can call GetHostname and receive the device hostname
- [ ] **DEV-07**: User can call GetNetworkInterfaces and receive network interface information

### Media Service

- [ ] **MEDIA-01**: User can call GetProfiles and receive at least one media profile with video source, encoder, and PTZ configuration references
- [ ] **MEDIA-02**: User can call GetStreamUri with a profile token and receive an RTSP URL as configured by the consumer
- [ ] **MEDIA-03**: User can call GetVideoSources and receive video source descriptions with resolution and frame rate
- [ ] **MEDIA-04**: User can call GetVideoSourceConfigurations and receive configurations linking video sources to profiles
- [ ] **MEDIA-05**: User can call GetVideoEncoderConfigurations and receive encoder settings (codec, resolution, bitrate)
- [ ] **MEDIA-06**: User can call GetSnapshotUri with a profile token and receive a snapshot URL as configured by the consumer

### PTZ Service

- [ ] **PTZ-01**: User can call GetNodes and receive PTZ node(s) advertising TranslationSpaceFov in RelativePanTiltTranslationSpace
- [ ] **PTZ-02**: User can call GetNode with a node token and receive the specific PTZ node details
- [ ] **PTZ-03**: User can call GetConfigurations and receive all PTZ configurations with node token references
- [ ] **PTZ-04**: User can call GetConfiguration with a config token and receive configuration details
- [ ] **PTZ-05**: User can call GetConfigurationOptions and receive supported PTZ spaces including TranslationSpaceFov with X/Y ranges
- [ ] **PTZ-06**: User can call GetServiceCapabilities and receive MoveStatus="true" capability advertisement
- [ ] **PTZ-07**: User can call RelativeMove with pan/tilt/zoom translation and the consumer's trait method is invoked with typed parameters
- [ ] **PTZ-08**: User can call ContinuousMove with velocity parameters and the consumer's trait method is invoked
- [ ] **PTZ-09**: User can call Stop and active PTZ movement ceases (PanTilt and Zoom booleans respected)
- [ ] **PTZ-10**: User can call GetStatus and receive MoveStatus (IDLE/MOVING) for PanTilt and Zoom
- [ ] **PTZ-11**: User can call GetPresets and receive the consumer's configured preset list
- [ ] **PTZ-12**: User can call GotoPreset with a preset token and the consumer's trait method is invoked
- [ ] **PTZ-13**: User can call AbsoluteMove with position parameters and the consumer's trait method is invoked
- [ ] **PTZ-14**: User can call SetPreset and the consumer's trait method is invoked to create/update a preset
- [ ] **PTZ-15**: User can call RemovePreset and the consumer's trait method is invoked to delete a preset

### Imaging Service

- [ ] **IMG-01**: User can call GetImagingSettings with a video source token and receive imaging settings from the consumer's trait implementation

### Events Service

- [ ] **EVT-01**: User can call GetEventProperties and receive supported event topics from the consumer's trait implementation
- [ ] **EVT-02**: User can call CreatePullPointSubscription and receive a subscription reference for polling events
- [ ] **EVT-03**: User can call PullMessages on a subscription and receive queued event notifications
- [ ] **EVT-04**: User can call Unsubscribe to terminate an event subscription

### Discovery

- [ ] **DISC-01**: When the `discovery` feature flag is enabled, the server responds to WS-Discovery Probe messages on UDP multicast 239.255.255.250:3702
- [ ] **DISC-02**: WS-Discovery ProbeMatch responses include the device's XAddrs and scopes

### Testing & Examples

- [ ] **TEST-01**: Integration test suite replaying Frigate's autotracker call sequence (GetProfiles → GetConfigurationOptions → GetServiceCapabilities → GetStatus → RelativeMove → GotoPreset)
- [ ] **TEST-02**: virtual_ptz example demonstrating a minimal consumer implementation with all required trait methods
- [ ] **TEST-03**: ONVIF Device Manager smoke test validating basic device discovery and info retrieval

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Extended Compatibility

- **COMPAT-01**: Media2 service (Profile T) with H.265 encoder configuration support
- **COMPAT-02**: Audio source and encoder configuration operations
- **COMPAT-03**: Profile G recording and search services

### Advanced Device Management

- **ADVDEV-01**: GetDNS, GetNetworkProtocols, GetDiscoveryMode operations
- **ADVDEV-02**: User management operations (CreateUser, DeleteUser)
- **ADVDEV-03**: SystemReboot, GetSystemLog operations

## Out of Scope

| Feature | Reason |
|---------|--------|
| RTSP server / video streaming | Scope explosion — this is a SOAP/ONVIF layer only. Consumer provides RTSP URL. |
| ONVIF client functionality | Different abstractions needed. Use lumeohq/onvif-rs for client use cases. |
| ONVIF conformance certification | Requires ONVIF membership and hardware test bench. Test against real clients instead. |
| Multi-camera-per-port routing | No current use case. Consumer runs one OnvifServer per camera. |
| Camera hardware drivers | This crate provides the server framework, not driver code. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| INFRA-01 | Phase 1 | Pending |
| INFRA-02 | Phase 1 | Pending |
| INFRA-03 | Phase 1 | Pending |
| INFRA-04 | Phase 1 | Pending |
| INFRA-05 | Phase 1 | Pending |
| INFRA-06 | Phase 1 | Pending |
| INFRA-07 | Phase 1 | Pending |
| INFRA-08 | Phase 1 | Pending |
| INFRA-09 | Phase 1 | Pending |
| DEV-01 | Phase 1 | Pending |
| DEV-02 | Phase 1 | Pending |
| DEV-03 | Phase 1 | Pending |
| DEV-04 | Phase 1 | Pending |
| DEV-05 | Phase 1 | Pending |
| DEV-06 | Phase 1 | Pending |
| DEV-07 | Phase 1 | Pending |
| MEDIA-01 | Phase 1 | Pending |
| MEDIA-02 | Phase 1 | Pending |
| MEDIA-03 | Phase 1 | Pending |
| MEDIA-04 | Phase 1 | Pending |
| MEDIA-05 | Phase 1 | Pending |
| MEDIA-06 | Phase 1 | Pending |
| PTZ-01 | Phase 1 | Pending |
| PTZ-02 | Phase 1 | Pending |
| PTZ-03 | Phase 1 | Pending |
| PTZ-04 | Phase 1 | Pending |
| PTZ-05 | Phase 1 | Pending |
| PTZ-06 | Phase 1 | Pending |
| PTZ-07 | Phase 1 | Pending |
| PTZ-08 | Phase 1 | Pending |
| PTZ-09 | Phase 1 | Pending |
| PTZ-10 | Phase 1 | Pending |
| PTZ-11 | Phase 1 | Pending |
| PTZ-12 | Phase 1 | Pending |
| PTZ-13 | Phase 1 | Pending |
| PTZ-14 | Phase 1 | Pending |
| PTZ-15 | Phase 1 | Pending |
| IMG-01 | Phase 1 | Pending |
| EVT-01 | Phase 1 | Pending |
| EVT-02 | Phase 1 | Pending |
| EVT-03 | Phase 1 | Pending |
| EVT-04 | Phase 1 | Pending |
| DISC-01 | Phase 1 | Pending |
| DISC-02 | Phase 1 | Pending |
| TEST-01 | Phase 1 | Pending |
| TEST-02 | Phase 1 | Pending |
| TEST-03 | Phase 1 | Pending |

**Coverage:**
- v1 requirements: 42 total
- Mapped to phases: 42
- Unmapped: 0 ✓

---
*Requirements defined: 2026-04-05*
*Last updated: 2026-04-05 after initial definition*
