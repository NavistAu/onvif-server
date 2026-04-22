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
pub use server::{BuildError, OnvifServer, OnvifServerBuilder};
pub use service::device::DeviceServiceHandler;
pub use service::events::EventServiceHandler;
pub use service::imaging::ImagingServiceHandler;
pub use service::media::MediaServiceHandler;
pub use service::ptz::PTZServiceHandler;
pub use soap_server::WsdlError;
pub use soap_server::WsdlLoader;
pub use traits::{DeviceService, EventService, ImagingService, MediaService, PTZService};
pub use wsdl_loader::EmbeddedWsdlLoader;
