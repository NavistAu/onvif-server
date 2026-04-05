use async_trait::async_trait;
use crate::error::{OnvifError, not_implemented};

/// ONVIF Imaging Service (Profile S).
///
/// All methods default to `not_implemented()`. Object-safe: store as `Arc<dyn ImagingService>`.
#[async_trait]
pub trait ImagingService: Send + Sync + 'static {
    /// Returns the imaging settings (brightness, contrast, etc.) for a video source.
    async fn get_imaging_settings(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Sets the imaging settings for a video source.
    async fn set_imaging_settings(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the imaging options (min/max ranges) for a video source.
    async fn get_options(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the current focus status of a video source.
    async fn get_move_options(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Moves the focus to an absolute, relative, or continuous position.
    async fn move_focus(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Stops any ongoing focus movement.
    async fn stop_focus(&self) -> Result<(), OnvifError> {
        not_implemented()
    }
}
