// Server implementation — full builder wired up in plan 03
use std::sync::Arc;
use crate::traits::DeviceService;

/// The running ONVIF server handle. Returned by OnvifServerBuilder::build() in plan 03.
pub struct OnvifServer;

/// Builder for configuring and starting the ONVIF server.
///
/// Declared here in plan 02 to validate that Arc<dyn DeviceService> compiles
/// (i.e., DeviceService is object-safe). Real fields and build() are wired in plan 03.
pub struct OnvifServerBuilder {
    pub port: u16,
    pub device_service: Option<Arc<dyn DeviceService>>,
}
