use crate::error::{not_implemented, OnvifError};
use crate::generated::types::{
    DeviceInfo, HostnameInformation, NetworkInterface, Scope, ScopeDefinition,
};
use async_trait::async_trait;

/// ONVIF Device Management Service (Profile S core).
///
/// All methods default to sensible values or `not_implemented()` — implementors
/// override only the operations their device supports. Trait is object-safe:
/// store as `Arc<dyn DeviceService>`.
///
/// NOTE: GetCapabilities and GetServices are framework-level — the handler
/// constructs those internally from the bound xaddr. Do NOT add them to this trait.
#[async_trait]
pub trait DeviceService: Send + Sync + 'static {
    /// Returns the current UTC time. Defaults to `chrono::Utc::now()`.
    async fn get_system_date_and_time(&self) -> Result<chrono::DateTime<chrono::Utc>, OnvifError> {
        Ok(chrono::Utc::now())
    }

    /// Returns manufacturer, model, firmware version, serial number, hardware ID.
    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        not_implemented()
    }

    /// Returns the scopes used for WS-Discovery advertisement.
    async fn get_scopes(&self) -> Result<Vec<Scope>, OnvifError> {
        Ok(vec![
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/type/video_encoder".into(),
            },
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/Profile/Streaming".into(),
            },
        ])
    }

    /// Returns the hostname of the device.
    async fn get_hostname(&self) -> Result<HostnameInformation, OnvifError> {
        Ok(HostnameInformation {
            from_dhcp: false,
            name: Some("onvif-device".into()),
        })
    }

    /// Returns network interface configurations.
    async fn get_network_interfaces(&self) -> Result<Vec<NetworkInterface>, OnvifError> {
        not_implemented()
    }
}
