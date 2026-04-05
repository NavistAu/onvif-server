use async_trait::async_trait;
use crate::error::{OnvifError, not_implemented};

/// ONVIF Media Service (Profile S).
///
/// Only get_stream_uri and get_snapshot_uri require consumer delegation.
/// All other operations (GetProfiles, GetVideoSources, etc.) are handler-internal
/// and return static responses built from token constants.
#[async_trait]
pub trait MediaService: Send + Sync + 'static {
    /// Returns the RTSP streaming URI for the given profile token.
    async fn get_stream_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        not_implemented()
    }

    /// Returns the HTTP snapshot URI for the given profile token.
    async fn get_snapshot_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        not_implemented()
    }
}
