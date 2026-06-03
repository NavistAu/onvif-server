# Services

`onvif-server` exposes five service traits. You implement the ones your hardware
needs; unimplemented methods return `OnvifError::NotImplemented`, which the SOAP
layer turns into a `ter:ActionNotSupported` fault (a well-formed fault, not a
dropped connection). This page is the how-to-implement guide; for the exact status
of every operation see [Operation Coverage](./coverage.md).

A note on the split between *trait* and *framework*: some responses are built by
the framework from builder config (capabilities, service list, discovery), and a
few are **static** canned XML (video source/encoder configs, the PTZ node tree).
Those are not on the traits — you cannot override them. The sections below cover
only what you implement.

---

## `DeviceService` — required

Mounted at `/onvif/device_service`, registered with `.device_service(impl)`.

| Method | Default | Implement when |
|--------|---------|----------------|
| `get_device_information` | faults | Always — clients call it early. Return manufacturer, model, firmware, serial, hardware id. |
| `get_system_date_and_time` | `Utc::now()` | Rarely. The default is correct for most devices. Always reported as UTC / `Manual`. |
| `get_scopes` | two fixed scopes | To customise the scopes returned by `GetScopes` (`video_encoder`, `Profile/Streaming` by default). |
| `get_hostname` | `"onvif-device"` | To report a real hostname. |
| `get_network_interfaces` | faults | Only if a client needs NIC enumeration (most do not). |

`GetCapabilities` and `GetServices` are **framework-built** from the services you
register and your `advertised_host` — not trait methods.

> **Scopes gotcha:** `get_scopes` sets the `GetScopes` response only. The scopes
> advertised over WS-Discovery are hardcoded (`NetworkVideoTransmitter`) and are
> *not* taken from this method. See [WS-Discovery](./discovery.md).

```rust,ignore
async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
    Ok(DeviceInfo {
        manufacturer: "Example Corp".into(),
        model: "EX-1".into(),
        firmware_version: "1.2.0".into(),
        serial_number: "SN-0001".into(),
        hardware_id: "ex-hw-1".into(),
    })
}
```

---

## `MediaService`

Mounted at `/onvif/media_service`, registered with `.media_service(impl)`.

| Method | Default | Implement when |
|--------|---------|----------------|
| `profiles` | one 1920×1080 H264 `MainProfile` (token `profile_0`) | To advertise real or multiple profiles. |
| `get_stream_uri` | faults | Always for streaming — return the RTSP URL for the given profile token. |
| `get_snapshot_uri` | faults | For JPEG snapshot support. |

The **profile token** you put in `profiles()` is the token clients pass back to
`get_stream_uri`/`get_snapshot_uri` — switch on it if you expose several profiles.

> **advertised_host vs stream URI:** `advertised_host` controls the host clients
> see in `GetCapabilities`/`GetServices` XAddrs. The stream/snapshot URIs are
> whatever *you* return here — they must independently point at a client-routable
> address (often the same host, RTSP port 554/8554).

`GetVideoSources`, `GetVideoSourceConfigurations`, and
`GetVideoEncoderConfigurations` are **static** (fixed 1920×1080 H264) and not
overridable; keep your advertised profile consistent with them for strict clients.

```rust,ignore
async fn get_stream_uri(&self, profile: &str) -> Result<String, OnvifError> {
    Ok(format!("rtsp://{}:554/{}", self.host, profile))
}
```

---

## `PTZService`

Mounted at `/onvif/ptz_service`, registered with `.ptz_service(impl)`.

Discovery operations (`GetNodes`, `GetNode`, `GetConfigurations`,
`GetConfiguration`, `GetConfigurationOptions`, `GetServiceCapabilities`) are
**static** and not on the trait. You implement the control surface:

| Method | Default | Notes |
|--------|---------|-------|
| `relative_move` / `absolute_move` / `continuous_move` | faults | Coordinates are floats; missing → `0.0`, malformed → fault before reaching you. |
| `stop` | faults | `pan_tilt` / `zoom` booleans; if the client omits both, both are `true`. |
| `get_status` | faults | Return `PTZStatusResult { pan_tilt_moving, zoom_moving }`; the response `UtcTime` is filled by the server. |
| `get_presets` | empty list | Return your saved presets. |
| `goto_preset` / `set_preset` / `remove_preset` | faults | `set_preset` returns the (new) preset token. |

> **Coordinate space:** the advertised node uses the field-of-view *relative*
> pan/tilt translation space with `XRange`/`YRange` of **−1..1** (and
> `MaximumNumberOfPresets` = 10). Interpret/clamp the `pan`/`tilt`/`zoom` arguments
> accordingly, and **never move real hardware on a malformed coordinate** — the
> framework already rejects unparseable values with a fault before calling you.

See the [`virtual_ptz`](./example-ptz.md) example for a full in-memory implementation.

---

## `ImagingService`

Mounted at `/onvif/imaging_service`, registered with `.imaging_service(impl)`.

| Method | Default | Notes |
|--------|---------|-------|
| `get_imaging_settings` | faults | Return an `ImagingSettings`; only `Some(_)` fields are emitted. |

White balance is special: set `white_balance_cr_gain` and/or
`white_balance_cb_gain` and the response emits a single `WhiteBalance` element with
`Mode=MANUAL` and the gain children. `SetImagingSettings` and the imaging options /
focus operations are **absent**.

```rust,ignore
async fn get_imaging_settings(&self, _token: String) -> Result<ImagingSettings, OnvifError> {
    Ok(ImagingSettings { brightness: Some(50.0), contrast: Some(50.0), ..Default::default() })
}
```

---

## `EventService`

Mounted at `/onvif/events_service`, registered with `.event_service(impl)`.

This service implements the WS-BaseNotification **pull-point lifecycle**
(`CreatePullPointSubscription`, `PullMessages`, `Unsubscribe`) entirely in the
framework, plus a static `GetEventProperties`. The single trait method,
`get_event_properties`, is currently **not consulted** by the handler.

> **Important limitation:** there is **no actual event delivery**. `PullMessages`
> validates the subscription and returns the current/termination time but never
> returns `NotificationMessage`s. Registering an `EventService` makes the
> subscription handshake succeed (so clients like Frigate don't error), but no
> motion/analytics events are pushed. Treat events as "subscribable but silent."

---

## Implementing incrementally

Start with `device_service` (`get_device_information`) + `media_service`
(`get_stream_uri`) — that is enough for most clients to enumerate the device and
open a stream. Add PTZ / imaging / events only as the client you target actually
calls them. Anything you skip faults cleanly with `ter:ActionNotSupported`.
