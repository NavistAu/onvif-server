use crate::constants::{PROFILE_TOKEN, VIDEO_SOURCE_TOKEN};
use crate::error::{not_implemented, OnvifError};
use crate::generated::types::MediaProfile;
use async_trait::async_trait;

/// ONVIF Media Service (Profile S).
///
/// Implement [`profiles`][MediaService::profiles] to advertise the profiles your
/// device actually supports. The default returns a single 1920×1080 H264 profile
/// so existing implementations continue to work unchanged.
///
/// Implement [`get_stream_uri`][MediaService::get_stream_uri] and
/// [`get_snapshot_uri`][MediaService::get_snapshot_uri] to supply URIs for
/// specific profile tokens.
#[async_trait]
pub trait MediaService: Send + Sync + 'static {
    /// Returns the media profiles this device supports.
    ///
    /// Override to expose multiple profiles or non-default resolutions/encoders.
    /// The default returns the single 1920×1080 H264 profile used by the
    /// static implementation, preserving backward compatibility.
    fn profiles(&self) -> Vec<MediaProfile> {
        vec![MediaProfile {
            token: PROFILE_TOKEN.to_string(),
            name: "MainProfile".to_string(),
            video_source_token: VIDEO_SOURCE_TOKEN.to_string(),
            width: 1920,
            height: 1080,
            encoding: "H264".to_string(),
            framerate: 30,
            bitrate: 4096,
        }]
    }

    /// Returns the RTSP streaming URI for the given profile token.
    async fn get_stream_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        not_implemented()
    }

    /// Returns the HTTP snapshot URI for the given profile token.
    async fn get_snapshot_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        not_implemented()
    }
}
