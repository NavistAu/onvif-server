# onvif-server

An ONVIF **Profile S streaming-core** device server library for Rust. Implement the service traits for your camera hardware to expose a device that standard ONVIF clients (Frigate, Home Assistant, ONVIF Device Manager, NVRs) can discover and stream from. See the coverage table below for exactly what is supported.

[![crates.io](https://img.shields.io/crates/v/onvif-server.svg)](https://crates.io/crates/onvif-server)
[![docs.rs](https://docs.rs/onvif-server/badge.svg)](https://docs.rs/onvif-server)
[![MSRV](https://img.shields.io/crates/msrv/onvif-server.svg)](https://crates.io/crates/onvif-server)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

---

## ONVIF Profile S coverage

| Service  | Status    |
|----------|-----------|
| Device   | Supported |
| Media    | Supported |
| PTZ      | Supported |
| Imaging  | Supported |
| Events   | Supported |

"Supported" means the service is routed and covers the **Profile S streaming
core** — not every operation in each ONVIF service. For exactly which operations
are implemented, which are backed by your trait, which return static responses,
their default behaviour, and which fault, see the
**[Operation Coverage matrix](https://navistau.github.io/onvif-server/coverage.html)**
and **[Capabilities & Limitations](https://navistau.github.io/onvif-server/capabilities.html)**.

---

## Features

| Feature     | Default | Description                                                           |
|-------------|---------|-----------------------------------------------------------------------|
| `discovery` | no      | WS-Discovery multicast listener on `239.255.255.250:3702` via `socket2` |

---

## Installation

```sh
cargo add onvif-server
```

### The `discovery` feature

To enable WS-Discovery (auto-discovery on the local network):

```sh
cargo add onvif-server --features discovery
```

### MSRV

See the MSRV badge above — the minimum supported Rust version is the `rust-version`
declared in the crate's `Cargo.toml`.

---

## Quick start

An empty `impl DeviceService for MyCamera {}` compiles, but a real client faults
immediately — `GetDeviceInformation` and `GetStreamUri` have no working default.
This is the smallest device a client can actually use (the
[`minimal_device`](examples/minimal_device.rs) example, runnable with
`cargo run --example minimal_device`):

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

`DeviceService` is the only **required** service — `.build()` returns
`Err(BuildError::MissingRequiredService)` if it is omitted. All other services
(Media, PTZ, Imaging, Events) are optional; unregistered services are simply not
advertised and their routes are not mounted. See the
[user guide](https://navistau.github.io/onvif-server/quickstart.html) for a
no-credentials `curl` smoke test, and the
[Operation Coverage matrix](https://navistau.github.io/onvif-server/coverage.html)
for what each operation does by default.

---

## Implementing service traits

All five traits (`DeviceService`, `MediaService`, `PTZService`, `ImagingService`,
`EventService`) provide default implementations for every method. Unoverridden
methods return `Err(OnvifError::NotImplemented)`, which the SOAP layer converts to a
well-formed SOAP fault with the ONVIF `ter:ActionNotSupported` subcode. Clients see a
standards-compliant fault rather than a connection error.

You can implement services incrementally: start with the methods your ONVIF client
actually calls and add more as needed.

---

## WS-Security

Call `.auth(username, password)` on the builder to enable WS-Security UsernameToken
digest authentication. When enabled, every SOAP request must carry a valid
`UsernameToken` header; requests without one receive a SOAP authentication fault.

`GetSystemDateAndTime` is automatically exempt from authentication, as required by
the ONVIF specification (clients must retrieve device time before they can compute a
valid digest).

When `.auth()` is **not** called the server runs unauthenticated and all operations
are accessible without credentials.

---

## WS-Discovery

Enable the `discovery` feature and the server spawns a background UDP listener when
`.run()` is called:

1. Joins IPv4 multicast group `239.255.255.250` on port `3702`.
2. Parses incoming datagrams; ignores anything that is not a well-formed WS-Discovery
   `Probe` message.
3. Responds with a `ProbeMatches` message embedding the device XAddr
   (`http://<advertised_host>:<port>/onvif/device_service`) and a stable
   EndpointReference UUID.

Use `.discovery_uuid(uuid::Uuid)` on the builder to supply a fixed UUID so the
device identity is stable across restarts. When not set, a random UUID-v4 is
generated at build time.

The probe-parsing and probe-response helpers (`discovery_is_probe`,
`discovery_build_probe_match`) are always compiled and available without the feature
flag, which makes them usable in tests.

---

## Example: virtual PTZ camera

The `virtual_ptz` example is a fully functional in-memory PTZ camera implementing all
five service traits. It demonstrates sharing state across multiple service
registrations using `Arc<Mutex<_>>`.

```
cargo run --example virtual_ptz
```

The server starts on port 8080 with credentials `admin`/`admin`. Connect any ONVIF
client (ONVIF Device Manager, VLC, Frigate, Home Assistant, python-onvif-zeep) to
`http://<host>:8080/onvif/device_service`.

---

## Documentation

- API reference: <https://docs.rs/onvif-server>
- User guide (mdBook): <https://navistau.github.io/onvif-server/>

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

The Rust source code in this repository is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT)), or
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

The bundled WSDL and XSD files under `wsdl/` are verbatim official ONVIF
specification documents and are **not** covered by the MIT/Apache-2.0 licenses above.
They are distributed under the ONVIF license; see [LICENSE-ONVIF](LICENSE-ONVIF) for
the full terms.

Copyright Joshua Hogendorn / NavistAu.
