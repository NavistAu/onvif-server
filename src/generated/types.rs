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

#[derive(Debug, Clone)]
pub struct MediaProfile {
    pub token: String,
    pub name: String,
    pub video_source_cfg_token: String,
    pub video_encoder_cfg_token: String,
    pub ptz_cfg_token: String,
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
