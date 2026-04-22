use crate::error::{not_implemented, OnvifError};
use async_trait::async_trait;

/// ONVIF Event Service (WS-BaseNotification pull-point pattern).
///
/// Only get_event_properties is a trait method. CreatePullPointSubscription,
/// PullMessages, and Unsubscribe are handler-internal state operations.
///
/// Object-safe: store as `Arc<dyn EventService>`.
#[async_trait]
pub trait EventService: Send + Sync + 'static {
    /// Returns all event topic namespaces supported by this device.
    /// The handler always returns a static minimal response regardless of this return value.
    async fn get_event_properties(&self) -> Result<(), OnvifError> {
        not_implemented()
    }
}
