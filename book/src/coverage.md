# Operation Coverage

This page lists every ONVIF operation `onvif-server` routes, how each is backed,
and what it does out of the box with an empty trait implementation. It is the
authoritative answer to "will my client work against this device?"

`onvif-server` targets the **Profile S streaming core** — enough for a client to
discover the device, enumerate a media profile, pull a stream/snapshot URI, drive
PTZ, and run a pull-point event subscription. It is **not** a full ONVIF device:
most management and configuration operations are absent.

## Legend

| Backing | Meaning |
|---------|---------|
| **Framework** | Response built internally by `onvif-server` from builder config (e.g. advertised host, registered services). No trait method; nothing to implement. |
| **Trait (default OK)** | Dispatched to a service-trait method that ships a working default. Usable without overriding; override to customise. |
| **Trait (override)** | Dispatched to a service-trait method whose default returns a `not_implemented` SOAP fault. **You must override it** for the operation to work. |
| **Static** | Returns fixed, canned XML. Not overridable and not driven by your trait data. |
| **Absent** | Not routed. Returns a `ter:ActionNotSupported` SOAP fault. |

"Default behaviour" = what happens when the service is registered but the trait
method is **not** overridden. ✅ = a usable response; ⚠️ = a response that may not
match your device; ❌ = a SOAP fault.

## Device — `tds`, route `/onvif/device_service`

Namespace `http://www.onvif.org/ver10/device/wsdl`. Always mounted (required).

| Operation | Backing | Default behaviour | Notes |
|-----------|---------|-------------------|-------|
| `GetSystemDateAndTime` | Trait (default OK) | ✅ `Utc::now()` | Auth-exempt (clock-sync before authenticating). Always reports `TimeZone=UTC`, `DateTimeType=Manual`. |
| `GetCapabilities` | Framework | ✅ | Lists only the services you registered, with their XAddrs. |
| `GetServices` | Framework | ✅ | Same set; version hardcoded to 2.42. |
| `GetScopes` | Trait (default OK) | ✅ two fixed scopes | Defaults to `video_encoder` + `Profile/Streaming`. Note: these are **not** the scopes advertised over WS-Discovery (see below). |
| `GetHostname` | Trait (default OK) | ✅ `"onvif-device"` | `FromDHCP=false`. |
| `GetDeviceInformation` | Trait (override) | ❌ fault | Override to return manufacturer/model/firmware/serial/hardware-id. |
| `GetNetworkInterfaces` | Trait (override) | ❌ fault | Override to advertise NICs. |
| *all other Device ops* | Absent | ❌ fault | `SetHostname`, `SetScopes`, `GetUsers`, `SystemReboot`, network/discovery config, etc. (~40 operations) are not routed. |

## Media — `trt`, route `/onvif/media_service`

Namespace `http://www.onvif.org/ver10/media/wsdl` (Media1). Optional. **Media2
(`ver20`) is not implemented.**

| Operation | Backing | Default behaviour | Notes |
|-----------|---------|-------------------|-------|
| `GetProfiles` | Trait (default OK) | ✅ one static profile | Default is a single 1920×1080 H264 `MainProfile`. Override `profiles()` to expose real/multiple profiles. |
| `GetStreamUri` | Trait (override) | ❌ fault | **Override this** — it is how clients learn the RTSP URL. |
| `GetSnapshotUri` | Trait (override) | ❌ fault | Override for JPEG snapshot support. |
| `GetVideoSources` | Static | ⚠️ fixed 1920×1080@30 | Canned; ignores your profile data. |
| `GetVideoSourceConfigurations` | Static | ⚠️ fixed | Canned. |
| `GetVideoEncoderConfigurations` | Static | ⚠️ fixed H264 1080p | Canned. May disagree with the profiles you advertise. |
| *all other Media ops* | Absent | ❌ fault | No `Set*`, `GetAudio*`, `GetOSDs`, `CreateProfile`, options, etc. |

## PTZ — `tptz`, route `/onvif/ptz_service`

Namespace `http://www.onvif.org/ver20/ptz/wsdl` (the `ver10` namespace is also
accepted on requests). Optional.

| Operation | Backing | Default behaviour | Notes |
|-----------|---------|-------------------|-------|
| `GetNodes` | Static | ✅ one node | FoV relative pan/tilt space, range −1..1, max 10 presets. |
| `GetNode` | Static | ✅ / fault | Faults on unknown node token. |
| `GetConfigurations` | Static | ✅ | Single `PTZConfig`. |
| `GetConfiguration` | Static | ✅ / fault | Faults on unknown config token. |
| `GetConfigurationOptions` | Static | ✅ | |
| `GetServiceCapabilities` | Static | ✅ | `MoveStatus=true` (attribute form, required by Frigate). |
| `GetPresets` | Trait (default OK) | ✅ empty list | Override to list saved presets. |
| `GetStatus` | Trait (override) | ❌ fault | `UtcTime` filled from the server clock. |
| `RelativeMove` | Trait (override) | ❌ fault | Coordinates parsed (absent → 0.0; malformed → fault). |
| `AbsoluteMove` | Trait (override) | ❌ fault | As above. |
| `ContinuousMove` | Trait (override) | ❌ fault | As above. |
| `Stop` | Trait (override) | ❌ fault | `PanTilt`/`Zoom` absent → both true. |
| `GotoPreset` | Trait (override) | ❌ fault | |
| `SetPreset` | Trait (override) | ❌ fault | Returns the preset token. |
| `RemovePreset` | Trait (override) | ❌ fault | |
| *all other PTZ ops* | Absent | ❌ fault | No `GotoHomePosition`/`SetHomePosition`, geo-move, presets-tours, etc. |

## Imaging — `timg`, route `/onvif/imaging_service`

Namespace `http://www.onvif.org/ver20/imaging/wsdl`. Optional.

| Operation | Backing | Default behaviour | Notes |
|-----------|---------|-------------------|-------|
| `GetImagingSettings` | Trait (override) | ❌ fault | Only the fields you set are emitted; white balance is reported as a single `MANUAL` element. |
| *all other Imaging ops* | Absent | ❌ fault | No `SetImagingSettings`, `GetOptions`, `GetMoveOptions`, focus `Move`/`Stop`/`GetStatus`. |

## Events — `tev` + WS-BaseNotification, route `/onvif/events_service`

Namespace `http://www.onvif.org/ver10/events/wsdl`. Optional. Implements the
pull-point **subscription lifecycle** only.

| Operation | Backing | Default behaviour | Notes |
|-----------|---------|-------------------|-------|
| `GetEventProperties` | Static | ✅ minimal | Fixed topic set with an **empty** `TopicSet`. The `get_event_properties` trait method is not consulted. |
| `CreatePullPointSubscription` | Framework | ✅ | Creates an in-memory subscription (UUID id, default 60 s lifetime; honours `InitialTerminationTime`). |
| `PullMessages` | Framework | ✅ **empty** | Validates the subscription and returns current/termination time but **no `NotificationMessage`s** — there is no actual event delivery. |
| `Unsubscribe` | Framework | ✅ | Removes the subscription. |
| *all other Events ops* | Absent | ❌ fault | No `Subscribe` (basic notify), renew, seek, etc. |

## WS-Discovery (UDP multicast `239.255.255.250:3702`)

Not an HTTP/SOAP service. **Gated behind the non-default `discovery` Cargo
feature.** When enabled, the server answers a `Probe` with `ProbeMatches`. It does
**not** send an unsolicited `Hello` on start.

| Behaviour | Backing | Notes |
|-----------|---------|-------|
| `Probe` → `ProbeMatches` | Framework | Replies to multicast `Probe` with the device-service XAddr and a fixed `Types`/`Scopes` set (`dn:NetworkVideoTransmitter`). These scopes are **hardcoded in discovery and independent of `DeviceService::get_scopes`.** See [WS-Discovery](./discovery.md). |

## What this is not

- **Profile S core only.** No Profile T/G/A/M/D/C operations.
- **Media1 only** (no Media2 / `ver20` media).
- **No configuration writes.** The only `Set*` operation implemented is PTZ
  `SetPreset`. Hostname, scopes, encoder config, imaging settings, users, network
  — none are settable.
- **No real event delivery.** The pull-point lifecycle works, but `PullMessages`
  never returns notifications.
- **Several responses are static.** Video source/encoder configs and the PTZ
  node/config tree are canned and can disagree with the profiles your trait
  advertises — keep them consistent in your implementation if a strict client
  cross-checks them.
