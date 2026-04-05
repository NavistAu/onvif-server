/// Token for the single media profile exposed by this device.
/// Used in GetProfiles, GetStreamUri, and all PTZ operations.
pub const PROFILE_TOKEN: &str = "profile_0";

/// Token for the single video source.
/// Referenced in VideoSourceConfiguration elements.
pub const VIDEO_SOURCE_TOKEN: &str = "video_src_0";

/// Token for the PTZ node describing the physical PTZ capabilities.
/// Referenced in GetNodes and GetNode responses.
pub const PTZ_NODE_TOKEN: &str = "ptz_node_0";

/// Token for the PTZ configuration attached to the media profile.
/// Used in GetConfiguration and PTZ service move operations.
pub const PTZ_CONFIG_TOKEN: &str = "ptz_cfg_0";

/// ONVIF PTZ translation space URI for field-of-view relative moves.
/// Used as the Space field in RelativeMove PanTilt arguments.
pub const TRANSLATION_SPACE_FOV: &str =
    "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov";
