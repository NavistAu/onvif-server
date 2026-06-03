> ⚠️ **Historical design document — not current API documentation.**
>
> This is a pre-implementation planning note written before `onvif-server` existed.
> It describes intended dependencies and APIs (`onvif-rs`/`yaserde` types, typed
> per-operation trait request/response signatures, a `.device_info(...)` builder)
> that do **not** match the shipped public API. The crate instead exposes
> handler traits per service over the `soap-server` transport. This file is kept
> only as a record of the original design intent.
>
> For accurate, current documentation see:
> - **README.md** (repo root) — overview, install, quick start.
> - **The mdBook** in [`book/`](../book/src/SUMMARY.md) — Introduction, Installation,
>   Quick Start, Operation Coverage, Services, WS-Security, WS-Discovery, examples.
> - **docs.rs/onvif-server** — the rustdoc API reference (the authoritative API).

# onvif-server: Rust ONVIF Device Server Crate

## Purpose

A Rust crate for building ONVIF-compliant device servers. Provides handler traits for each ONVIF service, bundles official WSDLs, and handles all ONVIF-specific protocol details. Built on the `soap-server` crate.

## License

MIT OR Apache-2.0 (dual licensed, standard Rust ecosystem convention).

## Dependencies

| Crate | Purpose |
|-------|---------|
| `soap-server` | SOAP transport, WSDL parsing, WS-Security (sibling project at `~/ws/soap-server`) |
| `onvif-rs` types (lumeohq) | XSD-generated request/response structs with yaserde |
| `yaserde` | XML serialization/deserialization for ONVIF types |
| `tokio` | Async runtime |
| `uuid` | Generate unique identifiers for device info |

## Bundled WSDLs

Official ONVIF WSDL and XSD files, shipped in the crate (freely distributable from onvif.org):

**Service WSDLs (subset relevant for Profile S + Profile T):**
- `devicemgmt.wsdl` — Device management (~40 operations)
- `media.wsdl` — Media service (~25 operations)
- `media2.wsdl` — Media2 service (~15 operations)
- `ptz.wsdl` — PTZ control (~15 operations)
- `imaging.wsdl` — Imaging settings (~10 operations)
- `events.wsdl` — Event service (~10 operations)
- `deviceio.wsdl` — Device I/O (~8 operations)

**Shared XSD schemas:**
- `onvif.xsd` — Core ONVIF types
- `types.xsd` — Common ONVIF types
- `ws-addr.xsd` — WS-Addressing
- `b-2.xsd` — WS-BaseNotification
- `bf-2.xsd` — WS-BrokeredNotification
- `ws-discovery.xsd` — WS-Discovery types

## Service Traits

Each ONVIF service is a Rust trait. Implementors override the methods they support; unimplemented methods return SOAP faults by default.

### DeviceService

```rust
#[async_trait]
pub trait DeviceService: Send + Sync + 'static {
    // Required for any ONVIF device
    async fn get_device_information(&self) -> Result<GetDeviceInformationResponse, OnvifError>;
    async fn get_capabilities(&self, req: GetCapabilitiesRequest) -> Result<GetCapabilitiesResponse, OnvifError>;
    async fn get_services(&self, req: GetServicesRequest) -> Result<GetServicesResponse, OnvifError>;
    async fn get_system_date_and_time(&self) -> Result<GetSystemDateAndTimeResponse, OnvifError>;
    async fn get_scopes(&self) -> Result<GetScopesResponse, OnvifError>;

    // Network
    async fn get_network_interfaces(&self) -> Result<GetNetworkInterfacesResponse, OnvifError> { not_implemented() }
    async fn get_dns(&self) -> Result<GetDNSResponse, OnvifError> { not_implemented() }
    async fn get_hostname(&self) -> Result<GetHostnameResponse, OnvifError> { not_implemented() }
    async fn get_network_protocols(&self) -> Result<GetNetworkProtocolsResponse, OnvifError> { not_implemented() }

    // Discovery
    async fn get_discovery_mode(&self) -> Result<GetDiscoveryModeResponse, OnvifError> { not_implemented() }

    // System
    async fn system_reboot(&self) -> Result<SystemRebootResponse, OnvifError> { not_implemented() }
    async fn get_system_log(&self, req: GetSystemLogRequest) -> Result<GetSystemLogResponse, OnvifError> { not_implemented() }

    // Users
    async fn get_users(&self) -> Result<GetUsersResponse, OnvifError> { not_implemented() }

    // ... all devicemgmt.wsdl operations with default not_implemented()
}
```

### MediaService

```rust
#[async_trait]
pub trait MediaService: Send + Sync + 'static {
    async fn get_profiles(&self) -> Result<GetProfilesResponse, OnvifError>;
    async fn get_stream_uri(&self, req: GetStreamUriRequest) -> Result<GetStreamUriResponse, OnvifError>;
    async fn get_video_sources(&self) -> Result<GetVideoSourcesResponse, OnvifError>;
    async fn get_video_source_configurations(&self) -> Result<GetVideoSourceConfigurationsResponse, OnvifError>;
    async fn get_snapshot_uri(&self, req: GetSnapshotUriRequest) -> Result<GetSnapshotUriResponse, OnvifError> { not_implemented() }

    // ... all media.wsdl operations with default not_implemented()
}
```

### PTZService

```rust
#[async_trait]
pub trait PTZService: Send + Sync + 'static {
    // Node/configuration discovery
    async fn get_nodes(&self) -> Result<GetNodesResponse, OnvifError>;
    async fn get_node(&self, req: GetNodeRequest) -> Result<GetNodeResponse, OnvifError>;
    async fn get_configurations(&self) -> Result<GetConfigurationsResponse, OnvifError>;
    async fn get_configuration(&self, req: GetConfigurationRequest) -> Result<GetConfigurationResponse, OnvifError>;
    async fn get_configuration_options(&self, req: GetConfigurationOptionsRequest) -> Result<GetConfigurationOptionsResponse, OnvifError>;
    async fn get_service_capabilities(&self) -> Result<GetServiceCapabilitiesResponse, OnvifError>;

    // Movement
    async fn relative_move(&self, req: RelativeMoveRequest) -> Result<RelativeMoveResponse, OnvifError>;
    async fn continuous_move(&self, req: ContinuousMoveRequest) -> Result<ContinuousMoveResponse, OnvifError>;
    async fn absolute_move(&self, req: AbsoluteMoveRequest) -> Result<AbsoluteMoveResponse, OnvifError> { not_implemented() }
    async fn stop(&self, req: StopRequest) -> Result<StopResponse, OnvifError>;
    async fn get_status(&self, req: GetStatusRequest) -> Result<GetStatusResponse, OnvifError>;

    // Presets
    async fn get_presets(&self, req: GetPresetsRequest) -> Result<GetPresetsResponse, OnvifError>;
    async fn goto_preset(&self, req: GotoPresetRequest) -> Result<GotoPresetResponse, OnvifError>;
    async fn set_preset(&self, req: SetPresetRequest) -> Result<SetPresetResponse, OnvifError> { not_implemented() }
    async fn remove_preset(&self, req: RemovePresetRequest) -> Result<RemovePresetResponse, OnvifError> { not_implemented() }

    // Home position
    async fn goto_home_position(&self, req: GotoHomePositionRequest) -> Result<GotoHomePositionResponse, OnvifError> { not_implemented() }
    async fn set_home_position(&self, req: SetHomePositionRequest) -> Result<SetHomePositionResponse, OnvifError> { not_implemented() }

    // ... all ptz.wsdl operations with default not_implemented()
}
```

### ImagingService

```rust
#[async_trait]
pub trait ImagingService: Send + Sync + 'static {
    async fn get_imaging_settings(&self, req: GetImagingSettingsRequest) -> Result<GetImagingSettingsResponse, OnvifError>;
    async fn get_move_options(&self, req: GetMoveOptionsRequest) -> Result<GetMoveOptionsResponse, OnvifError> { not_implemented() }
    async fn r#move(&self, req: MoveRequest) -> Result<MoveResponse, OnvifError> { not_implemented() }
    async fn stop(&self, req: StopRequest) -> Result<StopResponse, OnvifError> { not_implemented() }
    async fn get_status(&self, req: GetStatusRequest) -> Result<GetStatusResponse, OnvifError> { not_implemented() }

    // ... all imaging.wsdl operations with default not_implemented()
}
```

### EventService

```rust
#[async_trait]
pub trait EventService: Send + Sync + 'static {
    async fn get_event_properties(&self) -> Result<GetEventPropertiesResponse, OnvifError> { not_implemented() }
    async fn create_pull_point_subscription(&self, req: CreatePullPointSubscriptionRequest) -> Result<CreatePullPointSubscriptionResponse, OnvifError> { not_implemented() }
    async fn pull_messages(&self, req: PullMessagesRequest) -> Result<PullMessagesResponse, OnvifError> { not_implemented() }
    async fn unsubscribe(&self, req: UnsubscribeRequest) -> Result<UnsubscribeResponse, OnvifError> { not_implemented() }

    // ... all events.wsdl operations with default not_implemented()
}
```

## ONVIF-Specific Protocol Details

### TranslationSpaceFov Advertisement

The PTZ node must advertise support for the FOV translation space in `GetNodes` and `GetConfigurationOptions`:

```xml
<SupportedPTZSpaces>
  <RelativePanTiltTranslationSpace>
    <URI>http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov</URI>
    <XRange><Min>-1.0</Min><Max>1.0</Max></XRange>
    <YRange><Min>-1.0</Min><Max>1.0</Max></YRange>
  </RelativePanTiltTranslationSpace>
</SupportedPTZSpaces>
```

### GetServiceCapabilities MoveStatus

Must return `MoveStatus: true` so clients like Frigate know they can poll `GetStatus`:

```xml
<Capabilities MoveStatus="true" StatusPosition="false"/>
```

### GetStatus Response

```xml
<PTZStatus>
  <MoveStatus>
    <PanTilt>IDLE</PanTilt>    <!-- or MOVING -->
    <Zoom>IDLE</Zoom>
  </MoveStatus>
  <Position>                    <!-- optional, can be omitted -->
    <PanTilt x="0.0" y="0.0"/>
    <Zoom x="0.0"/>
  </Position>
</PTZStatus>
```

### Auth Exemptions

Per the ONVIF specification, `GetSystemDateAndTime` must be accessible without authentication. Clients use it to synchronize clocks before computing WS-Security digests. The onvif-server automatically registers this as auth-exempt with soap-server.

### ONVIF Operations Frigate Calls

Frigate (the primary consumer for the first application of this crate) uses python-onvif-zeep and calls these operations during autotracking:

| Method | Purpose | Frequency |
|--------|---------|-----------|
| `GetProfiles` | Enumerate media profiles | Once at startup |
| `GetConfigurationOptions` | Discover TranslationSpaceFov support | Once at startup |
| `GetServiceCapabilities` | Check MoveStatus support | Once at startup |
| `GetStatus` | Poll MOVING/IDLE | Frequent during tracking |
| `RelativeMove` | FOV-relative pan/tilt (core tracking command) | Per detection frame |
| `AbsoluteMove` | Zoom positioning | Occasional |
| `ContinuousMove` / `Stop` | Manual UI PTZ control | User-initiated |
| `GetPresets` / `GotoPreset` | Return to home after tracking | Start/end of tracking |
| `GetVideoSources` / `GetImagingSettings` | Zoom/focus info | Occasional |

## Builder Pattern

```rust
use onvif_server::{OnvifServer, DeviceInfo};

let server = OnvifServer::builder()
    .device_info(DeviceInfo {
        manufacturer: "NavistAu".into(),
        model: "Virtual PTZ".into(),
        firmware_version: env!("CARGO_PKG_VERSION").into(),
        serial_number: "VS-001".into(),
        hardware_id: "onvif-server-1".into(),
    })
    .device_service(my_device_impl)
    .media_service(my_media_impl)
    .ptz_service(my_ptz_impl)
    .imaging_service(my_imaging_impl)
    .auth("admin", "password123")
    .port(8080)
    .build()?;

server.run().await?;
```

## WS-Discovery (Optional Feature)

Behind a cargo feature flag `discovery`:

```toml
[features]
default = []
discovery = ["socket2"]
```

When enabled, the server announces itself on the network via UDP multicast on `239.255.255.250:3702`. Responds to Probe messages with ProbeMatch containing the device's XAddrs (ONVIF service URLs).

Not required for Frigate (which uses direct URL config) but useful for NVRs that auto-discover cameras (Home Assistant, Synology, AgentDVR).

## Type Definitions

The onvif-server crate depends on type definitions for all ONVIF request/response structures. Two approaches, to be determined during implementation:

**Option A: Depend on lumeohq/onvif-rs schema crate.** These types are already generated from official ONVIF XSDs using xsd-parser-rs + yaserde. They work bidirectionally (serialize and deserialize). If the types work well for server use, this is the fastest path.

**Option B: Generate our own types.** Use xsd-parser-rs directly against the bundled ONVIF XSD files in a `build.rs` step. This gives us control over the generated code and removes the external dependency, but duplicates work.

Decision deferred to implementation — try Option A first, fall back to B if needed.

## Crate Structure

```
onvif-server/
    src/
        lib.rs              # Public API, re-exports
        server.rs           # OnvifServer builder, wiring traits to soap-server
        error.rs            # OnvifError type
        traits/
            mod.rs          # Trait re-exports
            device.rs       # DeviceService trait
            media.rs        # MediaService trait
            ptz.rs          # PTZService trait
            imaging.rs      # ImagingService trait
            events.rs       # EventService trait
        discovery.rs        # WS-Discovery (feature-gated)
    wsdl/                   # Bundled ONVIF WSDLs and XSDs
        devicemgmt.wsdl
        media.wsdl
        ptz.wsdl
        imaging.wsdl
        events.wsdl
        onvif.xsd
        types.xsd
        ...
    tests/
        frigate_compat.rs   # Test Frigate's specific ONVIF call sequence
        onvif_dm_compat.rs  # Test against ONVIF Device Manager expectations
    examples/
        virtual_ptz.rs      # Minimal virtual PTZ camera example
```

## Testing Strategy

- Implement a minimal virtual PTZ camera and test against:
  - Frigate's python-onvif-zeep client (the actual consumer)
  - ONVIF Device Manager (widely used ONVIF test tool)
- Test the exact ONVIF call sequence Frigate's autotracker makes (from `frigate/ptz/onvif.py`)
- Verify TranslationSpaceFov advertisement is correctly detected
- Verify MoveStatus polling works as Frigate expects

## Context

onvif-server depends on soap-server for all SOAP transport, WSDL parsing, and WS-Security. onvif-server adds the ONVIF-specific layer: service traits, bundled WSDLs, protocol details, and type definitions.

Dependency chain: `soap-server` <- `onvif-server`

Both crates are published to crates.io under NavistAu ownership.
