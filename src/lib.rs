mod error;
mod constants;
mod server;
mod wsdl_loader;
#[cfg(feature = "discovery")]
pub(crate) mod discovery;
pub mod traits;
pub mod generated;
pub mod service;

pub use error::OnvifError;
pub use server::{OnvifServer, OnvifServerBuilder, BuildError};
pub use constants::*;
pub use soap_server::WsdlLoader;
pub use soap_server::WsdlError;
pub use generated::{
    DeviceInfo, Scope, ScopeDefinition, HostnameInformation, NetworkInterface,
    MediaProfile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration,
    PTZStatusResult, PTZPreset, ImagingSettings,
};
pub use wsdl_loader::EmbeddedWsdlLoader;
pub use traits::{DeviceService, MediaService, PTZService, ImagingService, EventService};
pub use service::device::DeviceServiceHandler;
pub use service::media::MediaServiceHandler;
pub use service::ptz::PTZServiceHandler;
pub use service::imaging::ImagingServiceHandler;
pub use service::events::EventServiceHandler;
