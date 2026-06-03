# Quick Start

The entry point is `OnvifServer::builder()`, which returns an `OnvifServerBuilder`.
Chain builder methods to configure the server, call `.build()` to validate and
construct the server, then call `.run().await` to bind the port and begin serving.

```rust,no_run
use onvif_server::{OnvifServer, DeviceService};

struct MyCamera;

#[async_trait::async_trait]
impl DeviceService for MyCamera {
    // Override methods as needed; defaults return NotImplemented.
}

#[tokio::main]
async fn main() {
    OnvifServer::builder()
        .port(8080)
        .advertised_host("192.168.1.10")
        .device_service(MyCamera)
        .auth("admin", "password")
        .build()
        .expect("build failed")
        .run()
        .await
        .expect("server error");
}
```

## Builder methods

### `.port(u16)`

The TCP port the server listens on. Defaults to `8080`.

### `.advertised_host(&str)`

The host address embedded in XAddr URLs returned to ONVIF clients via
`GetCapabilities`, `GetServices`, and WS-Discovery responses.

ONVIF clients use these URLs to make follow-up requests, so this must be a
routable address from the client's perspective — for example `"192.168.1.10"`,
not `"0.0.0.0"`. Defaults to `"0.0.0.0"` for backward compatibility.

### `.device_service(impl DeviceService)`

Registers the Device Management Service implementation. This is the **only
required service** — `.build()` returns `Err(BuildError::MissingRequiredService)`
if it is omitted. The device service handles core ONVIF operations such as
`GetSystemDateAndTime`, `GetCapabilities`, `GetDeviceInformation`, and others.

### `.media_service(impl MediaService)`

Registers the Media Service implementation. Optional — if omitted, the
`/onvif/media_service` route is not mounted and media capabilities are not
advertised in `GetCapabilities`.

### `.ptz_service(impl PTZService)`

Registers the PTZ Service implementation. Optional — if omitted, the
`/onvif/ptz_service` route is not mounted.

### `.imaging_service(impl ImagingService)`

Registers the Imaging Service implementation. Optional — if omitted, the
`/onvif/imaging_service` route is not mounted.

### `.event_service(impl EventService)`

Registers the Event Service implementation. Optional — if omitted, the
`/onvif/events_service` route is not mounted.

### `.auth(&str, &str)`

Enables WS-Security UsernameToken digest authentication with the given
username and password. When this method is **not** called, the server runs
unauthenticated and all operations are accessible without credentials.
See [WS-Security](./ws-security.md) for details.

### `.discovery_uuid(uuid::Uuid)`

Overrides the stable WS-Discovery EndpointReference UUID for this device.
When not called, a random UUID-v4 is used at build time. Callers that need a
deterministic identity across restarts should supply a stable UUID derived from
hardware ID or stored configuration.

### `.build()`

Validates configuration and constructs the `OnvifServer`. Returns
`Err(BuildError::MissingRequiredService("device_service"))` if no device service
was registered.

### `.run().await`

Binds `0.0.0.0:<port>` and starts serving SOAP requests. Does not return until
the server shuts down. Requires a tokio async runtime. Returns `RunError::Io` on
TCP bind failure.
