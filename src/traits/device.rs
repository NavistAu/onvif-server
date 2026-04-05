use async_trait::async_trait;
use crate::error::{OnvifError, not_implemented};

/// ONVIF Device Management Service (Profile S core).
///
/// All methods default to `not_implemented()` — implementors override only the
/// operations their device supports. Trait is object-safe: store as `Arc<dyn DeviceService>`.
#[async_trait]
pub trait DeviceService: Send + Sync + 'static {
    /// Returns the device clock and timezone. Required by all ONVIF profiles.
    async fn get_system_date_and_time(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the service capabilities for this device.
    async fn get_capabilities(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns manufacturer, model, firmware version, serial number, hardware ID.
    async fn get_device_information(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns a list of all supported services and their version numbers.
    async fn get_services(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the scopes used for WS-Discovery advertisement.
    async fn get_scopes(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the list of NTP servers configured on the device.
    async fn get_ntp(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the hostname of the device.
    async fn get_hostname(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns user credentials stored on the device.
    async fn get_users(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the system URI for firmware upgrade or log access.
    async fn get_system_uris(&self) -> Result<(), OnvifError> {
        not_implemented()
    }
}
