// Auto-generated ONVIF type stubs — Phase 1 skeleton
// Full XSD-derived codegen is a Phase 2+ concern.
// These stubs satisfy INFRA-04 so Phase 2 can build against concrete types immediately.

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeDefinition {
    Fixed,
    Configurable,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub scope_def: ScopeDefinition,
    pub scope_item: String,
}

#[derive(Debug, Clone)]
pub struct HostnameInformation {
    pub from_dhcp: bool,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub token: String,
    pub enabled: bool,
    pub name: String,
    pub hw_address: String,
    pub mtu: u32,
}

/// A media profile as advertised by the ONVIF Media Service.
///
/// Implementations override [`crate::traits::MediaService::profiles`] to return
/// their actual profiles. The default returns a single 1920×1080 H264 profile.
#[derive(Debug, Clone)]
pub struct MediaProfile {
    /// Profile token referenced by GetStreamUri, GetSnapshotUri, and PTZ operations.
    pub token: String,
    /// Human-readable profile name.
    pub name: String,
    /// Source token for the VideoSourceConfiguration element.
    pub video_source_token: String,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Encoder codec, e.g. `"H264"`, `"H265"`, `"JPEG"`.
    pub encoding: String,
    /// Target frame rate in frames per second.
    pub framerate: u32,
    /// Target bitrate in kbps.
    pub bitrate: u32,
}

#[derive(Debug, Clone)]
pub struct VideoSource {
    pub token: String,
    pub framerate: f32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct VideoSourceConfiguration {
    pub token: String,
    pub name: String,
    pub source_token: String,
    pub bounds_x: i32,
    pub bounds_y: i32,
    pub bounds_width: i32,
    pub bounds_height: i32,
}

#[derive(Debug, Clone)]
pub struct VideoEncoderConfiguration {
    pub token: String,
    pub name: String,
    pub encoding: String,
    pub width: i32,
    pub height: i32,
    pub framerate: i32,
    pub bitrate: i32,
}

/// Returned by PTZService::get_status(). Handler serializes to nested PTZMoveStatus XML.
#[derive(Debug, Clone)]
pub struct PTZStatusResult {
    pub pan_tilt_moving: bool,
    pub zoom_moving: bool,
}

/// One PTZ preset returned by PTZService::get_presets(). Handler serializes to PTZPreset XML.
#[derive(Debug, Clone)]
pub struct PTZPreset {
    pub token: String,
    pub name: String,
}

/// Returned by ImagingService::get_imaging_settings(). Handler serializes to ImagingSettings XML.
/// Only Some fields are emitted — None fields produce no XML element.
#[derive(Debug, Clone, Default)]
pub struct ImagingSettings {
    pub brightness: Option<f32>,
    pub color_saturation: Option<f32>,
    pub contrast: Option<f32>,
    pub sharpness: Option<f32>,
    pub white_balance_cr_gain: Option<f32>,
    pub white_balance_cb_gain: Option<f32>,
}
