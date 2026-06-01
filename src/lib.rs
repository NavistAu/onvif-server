//! # onvif-server
//!
//! A spec-compliant ONVIF Profile S device server library for Rust.
//!
//! Implement the service traits for your camera hardware and get a fully functional
//! ONVIF-compatible device accessible by any standard ONVIF client (VMS, NVR,
//! Home Assistant, Frigate, python-onvif-zeep, ONVIF Device Manager, etc.).
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
//! ```rust,no_run
//! use onvif_server::{OnvifServer, DeviceService};
//!
//! struct MyCamera;
//!
//! #[async_trait::async_trait]
//! impl DeviceService for MyCamera {
//!     // Override methods as needed; defaults return NotImplemented.
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     OnvifServer::builder()
//!         .port(8080)
//!         .advertised_host("192.168.1.10")
//!         .device_service(MyCamera)
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
#[cfg(feature = "discovery")]
pub(crate) mod discovery;
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
