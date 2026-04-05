use async_trait::async_trait;
use crate::error::{OnvifError, not_implemented};

/// ONVIF Event Service (WS-BaseNotification pull-point pattern).
///
/// All methods default to `not_implemented()`. Object-safe: store as `Arc<dyn EventService>`.
#[async_trait]
pub trait EventService: Send + Sync + 'static {
    /// Creates a pull-point subscription and returns its reference URI.
    async fn create_pull_point_subscription(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Pulls pending event messages from an active pull-point subscription.
    async fn pull_messages(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Unsubscribes and destroys a pull-point subscription.
    async fn unsubscribe(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Renews a subscription to extend its lifetime.
    async fn renew(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the event service capabilities.
    async fn get_service_capabilities(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns all event topic namespaces supported by this device.
    async fn get_event_properties(&self) -> Result<(), OnvifError> {
        not_implemented()
    }
}
