//! # onvif-server
//!
//! An ONVIF **Profile S streaming-core** device server library for Rust.
//!
//! Implement the service traits for your camera hardware to expose a device that
//! standard ONVIF clients (VMS, NVR, Home Assistant, Frigate, python-onvif-zeep,
//! ONVIF Device Manager) can discover and stream from. It targets the Profile S
//! streaming core, not every ONVIF operation — see the Operation Coverage matrix
//! in the user guide for the exact support claims.
//!
//! ## ONVIF Profile S coverage
//!
//! | Service        | Status        |
//! |----------------|---------------|
//! | Device         | Supported     |
//! | Media          | Supported     |
//! | PTZ            | Supported     |
//! | Imaging        | Supported     |
//! | Events         | Supported     |
//!
//! ## Quick start
//!
//! An empty `impl DeviceService for MyCamera {}` compiles but faults on the first
//! real request — `GetDeviceInformation` and `GetStreamUri` have no working
//! default. This is the smallest *usable* device (see the `minimal_device`
//! example, and the Operation Coverage matrix in the user guide for what each
//! operation does by default):
//!
//! ```rust,no_run
//! use async_trait::async_trait;
//! use onvif_server::{DeviceInfo, DeviceService, MediaService, OnvifError, OnvifServer};
//!
//! #[derive(Clone)]
//! struct MinimalCamera {
//!     media_host: String,
//! }
//!
//! #[async_trait]
//! impl DeviceService for MinimalCamera {
//!     async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
//!         Ok(DeviceInfo {
//!             manufacturer: "Example Corp".into(),
//!             model: "Minimal-1".into(),
//!             firmware_version: "1.0.0".into(),
//!             serial_number: "SN-0001".into(),
//!             hardware_id: "minimal-hw-1".into(),
//!         })
//!     }
//! }
//!
//! #[async_trait]
//! impl MediaService for MinimalCamera {
//!     async fn get_stream_uri(&self, _profile: &str) -> Result<String, OnvifError> {
//!         Ok(format!("rtsp://{}:8554/stream", self.media_host))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let host = "192.168.1.10";
//!     let cam = MinimalCamera { media_host: host.into() };
//!     OnvifServer::builder()
//!         .port(8080)
//!         .advertised_host(host)
//!         .device_service(cam.clone())
//!         .media_service(cam)
//!         .auth("admin", "password")
//!         .build()
//!         .expect("build failed")
//!         .run()
//!         .await
//!         .expect("server error");
//! }
//! ```
//!
//! ## WS-Security
//!
//! Call `.auth(username, password)` on the builder to enable WS-Security
//! UsernameToken digest authentication. `GetSystemDateAndTime` is automatically
//! exempt from auth (required by ONVIF spec for clock synchronisation before
//! the client has valid credentials).
//!
//! ## WS-Discovery
//!
//! Enable the optional `discovery` feature to have the server respond to
//! WS-Discovery multicast probes on `239.255.255.250:3702`, making the device
//! auto-discoverable on the local network.
//!
//! ```toml
//! [dependencies]
//! onvif-server = { version = "0.1", features = ["discovery"] }
//! ```

mod constants;
pub mod discovery;
mod error;
pub mod generated;
mod server;
pub mod service;
pub mod traits;
mod wsdl_loader;

pub use constants::*;
pub use error::OnvifError;
pub use generated::{
    DeviceInfo, HostnameInformation, ImagingSettings, MediaProfile, NetworkInterface, PTZPreset,
    PTZStatusResult, Scope, ScopeDefinition, VideoEncoderConfiguration, VideoSource,
    VideoSourceConfiguration,
};
pub use server::{BuildError, OnvifServer, OnvifServerBuilder, RunError};
pub use service::device::DeviceServiceHandler;
pub use service::events::EventServiceHandler;
pub use service::imaging::ImagingServiceHandler;
pub use service::media::MediaServiceHandler;
pub use service::ptz::PTZServiceHandler;
pub use soap_server::WsdlError;
pub use soap_server::WsdlLoader;
pub use traits::{DeviceService, EventService, ImagingService, MediaService, PTZService};
pub use wsdl_loader::EmbeddedWsdlLoader;

// ─── Discovery helpers ────────────────────────────────────────────────────────
//
// Thin wrappers exposing the WS-Discovery probe-parsing and probe-response
// building functions. These are always available (the underlying logic is pure
// XML); only the UDP listener requires the `discovery` feature.

/// Returns `true` when `msg` is a well-formed WS-Discovery `Probe` message
/// (SOAP body first child = `Probe` in namespace
/// `http://schemas.xmlsoap.org/ws/2005/04/discovery`).
pub fn discovery_is_probe(msg: &[u8]) -> bool {
    discovery::is_probe_message(msg)
}

/// Build a WS-Discovery `ProbeMatches` response XML with a stable
/// `device_uuid` embedded in `EndpointReference/Address`.
pub fn discovery_build_probe_match(
    relates_to: &str,
    xaddr: &str,
    device_uuid: uuid::Uuid,
) -> String {
    discovery::build_probe_match(relates_to, xaddr, device_uuid)
}
