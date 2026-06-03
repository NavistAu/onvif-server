# Example: Virtual PTZ Camera

The `virtual_ptz` example (`examples/virtual_ptz.rs`) is a minimal, fully functional
in-memory PTZ camera that implements all five ONVIF service traits. It demonstrates
how to share state across multiple service registrations using `Arc<Mutex<_>>`.

## Running the example

```
cargo run --example virtual_ptz
```

The server binds on port 8080 and prints its service URLs:

```
Virtual PTZ ONVIF server running on :8080
  Device service:  http://0.0.0.0:8080/onvif/device_service
  Media service:   http://0.0.0.0:8080/onvif/media_service
  PTZ service:     http://0.0.0.0:8080/onvif/ptz_service
  Imaging service: http://0.0.0.0:8080/onvif/imaging_service
  Events service:  http://0.0.0.0:8080/onvif/events_service
  Credentials:     admin / admin
```

Connect any ONVIF client (ONVIF Device Manager, VLC, Frigate, Home Assistant) to
`http://<host>:8080/onvif/device_service` with username `admin` and password `admin`.

## What the example builds

### Shared state: `VirtualPTZ`

```rust,no_run
#[derive(Clone)]
struct VirtualPTZ {
    presets: Arc<Mutex<HashMap<String, String>>>,
    preset_counter: Arc<Mutex<u32>>,
}
```

`VirtualPTZ` stores PTZ presets in memory. It is `Clone`, so a single instance
can be registered for multiple service slots without wrapping in another `Arc` —
the internal `Arc`s are what actually share state between the clones.

### `DeviceService`

Returns a static `DeviceInfo` with manufacturer `"Virtual"`, model `"VirtualPTZ"`,
firmware `"1.0"`, serial number `"0000"`, and hardware ID `"virtual-hw-0"`.

### `MediaService`

Returns fixed URIs:

- Stream URI: `rtsp://127.0.0.1:8554/stream`
- Snapshot URI: `http://127.0.0.1:8080/snapshot.jpg`

### `PTZService`

Implements the full PTZ surface:

| Method | Behaviour |
|--------|-----------|
| `relative_move(profile, pan, tilt, zoom)` | Logs the move; no hardware. |
| `absolute_move(profile, pan, tilt, zoom)` | Logs the move; no hardware. |
| `continuous_move(profile, pan, tilt, zoom)` | Logs the move; no hardware. |
| `stop(profile, pan_tilt, zoom)` | Logs the stop; no hardware. |
| `get_status(profile)` | Returns `pan_tilt_moving: false, zoom_moving: false`. |
| `get_presets(profile)` | Returns all presets from the in-memory map. |
| `set_preset(profile, name, token)` | Inserts into the map; auto-generates a token if none supplied. |
| `goto_preset(profile, token)` | Logs the goto; no hardware. |
| `remove_preset(profile, token)` | Removes from the map. |

PTZ presets are lost on restart.

### `ImagingService`

Returns static `ImagingSettings` with brightness, contrast, and sharpness each
set to `50.0`.

### `EventService`

Uses the default (all methods return `NotImplemented`). ONVIF clients that
request event subscriptions receive a SOAP fault.

## Server assembly

All five service slots are registered using clones of the same `VirtualPTZ`:

```rust,no_run
let cam = VirtualPTZ::new();

let server = onvif_server::OnvifServer::builder()
    .port(8080)
    .auth("admin", "admin")
    .device_service(cam.clone())
    .media_service(cam.clone())
    .ptz_service(cam.clone())
    .imaging_service(cam.clone())
    .event_service(cam)
    .build()?;

server.run().await?;
```

Because `VirtualPTZ` holds `Arc<Mutex<_>>` internally, all five registered clones
share the same preset storage.

## What an ONVIF client sees

- A fully enumerable device with `GetCapabilities` advertising all five services.
- A media profile with a stream URI pointing to a local RTSP address.
- Full PTZ control surfaces (move, stop, preset CRUD).
- Imaging settings query support.
- An events endpoint that responds with `ActionNotSupported` for subscription
  requests (the default `EventService` implementation).
