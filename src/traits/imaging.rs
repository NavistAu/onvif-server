use crate::error::{not_implemented, OnvifError};
use crate::generated::ImagingSettings;
use async_trait::async_trait;

/// ONVIF Imaging Service (Profile S).
///
/// All methods default to `not_implemented()`. Object-safe: store as `Arc<dyn ImagingService>`.
#[async_trait]
pub trait ImagingService: Send + Sync + 'static {
    /// Returns the imaging settings (brightness, contrast, etc.) for a video source.
    async fn get_imaging_settings(
        &self,
        video_source_token: String,
    ) -> Result<ImagingSettings, OnvifError> {
        let _ = video_source_token;
        not_implemented()
    }
}
