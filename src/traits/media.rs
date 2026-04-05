use async_trait::async_trait;
use crate::error::{OnvifError, not_implemented};

/// ONVIF Media Service (Profile S).
///
/// All methods default to `not_implemented()`. Object-safe: store as `Arc<dyn MediaService>`.
#[async_trait]
pub trait MediaService: Send + Sync + 'static {
    /// Returns all media profiles configured on the device.
    async fn get_profiles(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns an RTSP streaming URI for a given profile token.
    async fn get_stream_uri(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns an HTTP URI for a JPEG snapshot.
    async fn get_snapshot_uri(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the video encoder configuration for a given token.
    async fn get_video_encoder_configuration(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns all available video encoder configurations.
    async fn get_video_encoder_configurations(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns all video source configurations.
    async fn get_video_source_configurations(&self) -> Result<(), OnvifError> {
        not_implemented()
    }
}
