# Quick Start

The entry point is `OnvifServer::builder()`, which returns an `OnvifServerBuilder`.
Chain builder methods to configure the server, call `.build()` to validate and
construct it, then call `.run().await` to bind the port and begin serving.

## A minimal *usable* device

An empty `impl DeviceService for MyCamera {}` compiles, but a real client faults
immediately — `GetDeviceInformation` and `GetStreamUri` have no working default.
The smallest device a client can actually use implements those few operations and
lets the framework handle the rest:

```rust,no_run
use async_trait::async_trait;
use onvif_server::{DeviceInfo, DeviceService, MediaService, OnvifError, OnvifServer};

#[derive(Clone)]
struct MinimalCamera {
    media_host: String, // the camera's routable IP, used in stream/snapshot URIs
}

#[async_trait]
impl DeviceService for MinimalCamera {
    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        Ok(DeviceInfo {
            manufacturer: "Example Corp".into(),
            model: "Minimal-1".into(),
            firmware_version: "1.0.0".into(),
            serial_number: "SN-0001".into(),
            hardware_id: "minimal-hw-1".into(),
        })
    }
    // get_scopes / get_hostname / get_system_date_and_time use working defaults.
}

#[async_trait]
impl MediaService for MinimalCamera {
    // profiles() defaults to one 1920x1080 H264 "MainProfile".
    async fn get_stream_uri(&self, _profile: &str) -> Result<String, OnvifError> {
        Ok(format!("rtsp://{}:8554/stream", self.media_host))
    }
    async fn get_snapshot_uri(&self, _profile: &str) -> Result<String, OnvifError> {
        Ok(format!("http://{}:8080/snapshot.jpg", self.media_host))
    }
}

#[tokio::main]
async fn main() {
    let host = "192.168.1.10"; // the address clients route to
    let cam = MinimalCamera { media_host: host.into() };

    OnvifServer::builder()
        .port(8080)
        .advertised_host(host)
        .device_service(cam.clone())
        .media_service(cam)
        .auth("admin", "password")
        .build()
        .expect("build failed")
        .run()
        .await
        .expect("server error");
}
```

This is the [`minimal_device`](https://github.com/NavistAu/onvif-server/blob/main/examples/minimal_device.rs)
example — run it with `cargo run --example minimal_device`. For a fuller device
that implements all five service traits (PTZ, imaging, events) see
[`virtual_ptz`](./example-ptz.md). For what each operation does and what still
faults by default, see [Operation Coverage](./coverage.md).

## Verify it is serving

`GetSystemDateAndTime` is auth-exempt (ONVIF requires it so clients can sync
clocks before authenticating), which makes it a perfect no-credentials smoke test:

```sh
curl -s http://192.168.1.10:8080/onvif/device_service \
  -H 'Content-Type: application/soap+xml' \
  -d '<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
        <s:Body>
          <GetSystemDateAndTime xmlns="http://www.onvif.org/ver10/device/wsdl"/>
        </s:Body>
      </s:Envelope>'
```

You should get a `GetSystemDateAndTimeResponse` with the current UTC time. A
client then authenticates (see [WS-Security](./ws-security.md)) and calls
`GetDeviceInformation`, `GetProfiles`, and `GetStreamUri` to reach the stream.

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
