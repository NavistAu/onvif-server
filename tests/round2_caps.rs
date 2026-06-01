/// Round-2 finding #8 — capabilities half.
///
/// Tests:
///   - multi-profile: GetProfiles returns all profiles from an overridden profiles()
///   - default-profile: GetProfiles still returns original MainProfile/1920×1080 (compat)
///   - advertise-registered: GetServices/GetCapabilities omit ptz/imaging/events when not registered
use bytes::Bytes;
use onvif_server::service::device::DeviceServiceHandler;
use onvif_server::service::media::MediaServiceHandler;
use onvif_server::{
    DeviceInfo, DeviceService, MediaProfile, MediaService, NetworkInterface, OnvifError,
};
use soap_server::SoapHandler;
use std::sync::Arc;

// ─── Stubs ────────────────────────────────────────────────────────────────────

struct StubDevice;

#[async_trait::async_trait]
impl DeviceService for StubDevice {
    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        Ok(DeviceInfo {
            manufacturer: "Test".into(),
            model: "Test".into(),
            firmware_version: "0".into(),
            serial_number: "0".into(),
            hardware_id: "0".into(),
        })
    }

    async fn get_network_interfaces(&self) -> Result<Vec<NetworkInterface>, OnvifError> {
        Ok(vec![])
    }
}

/// MediaService that returns TWO profiles with different tokens/resolutions.
struct DualProfileMedia;

#[async_trait::async_trait]
impl MediaService for DualProfileMedia {
    fn profiles(&self) -> Vec<MediaProfile> {
        vec![
            MediaProfile {
                token: "profile_hd".to_string(),
                name: "HD".to_string(),
                video_source_token: "video_src_0".to_string(),
                width: 1920,
                height: 1080,
                encoding: "H264".to_string(),
                framerate: 30,
                bitrate: 4096,
            },
            MediaProfile {
                token: "profile_sd".to_string(),
                name: "SD".to_string(),
                video_source_token: "video_src_0".to_string(),
                width: 640,
                height: 480,
                encoding: "H264".to_string(),
                framerate: 15,
                bitrate: 512,
            },
        ]
    }

    async fn get_stream_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("rtsp://cam/{profile_token}"))
    }

    async fn get_snapshot_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("http://cam/{profile_token}/snap.jpg"))
    }
}

/// Default MediaService — no profiles() override.
struct DefaultMedia;

#[async_trait::async_trait]
impl MediaService for DefaultMedia {
    async fn get_stream_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("rtsp://cam/{profile_token}"))
    }

    async fn get_snapshot_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("http://cam/{profile_token}/snap.jpg"))
    }
}

// ─── Part A: trait-supplied profiles ─────────────────────────────────────────

/// An overridden profiles() returning TWO profiles must produce BOTH profile tokens
/// and BOTH resolutions in the GetProfiles response.
#[tokio::test]
async fn multi_profile_get_profiles_returns_both() {
    let handler = MediaServiceHandler::new(
        Arc::new(DualProfileMedia),
        "http://localhost:8080/onvif/media_service",
    );
    let body = Bytes::from_static(
        b"<trt:GetProfiles xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetProfiles must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    // Both profile tokens must appear.
    assert!(
        xml.contains("profile_hd"),
        "HD profile token missing: {xml}"
    );
    assert!(
        xml.contains("profile_sd"),
        "SD profile token missing: {xml}"
    );

    // HD resolution.
    assert!(
        xml.contains("<tt:Width>1920</tt:Width>"),
        "HD width 1920 missing: {xml}"
    );
    assert!(
        xml.contains("<tt:Height>1080</tt:Height>"),
        "HD height 1080 missing: {xml}"
    );

    // SD resolution.
    assert!(
        xml.contains("<tt:Width>640</tt:Width>"),
        "SD width 640 missing: {xml}"
    );
    assert!(
        xml.contains("<tt:Height>480</tt:Height>"),
        "SD height 480 missing: {xml}"
    );

    // Two Profiles elements.
    let count = xml.matches("<trt:Profiles ").count();
    assert_eq!(
        count, 2,
        "Expected 2 trt:Profiles elements, got {count}: {xml}"
    );
}

/// The default (non-overriding) MediaService must still return exactly the original
/// MainProfile at 1920×1080 with token profile_0 — backward compatibility.
#[tokio::test]
async fn default_profile_backward_compat() {
    let handler = MediaServiceHandler::new(
        Arc::new(DefaultMedia),
        "http://localhost:8080/onvif/media_service",
    );
    let body = Bytes::from_static(
        b"<trt:GetProfiles xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetProfiles must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert!(
        xml.contains("profile_0"),
        "Default profile_0 token missing: {xml}"
    );
    assert!(
        xml.contains("MainProfile"),
        "Default MainProfile name missing: {xml}"
    );
    assert!(
        xml.contains("<tt:Width>1920</tt:Width>"),
        "Default 1920 width missing: {xml}"
    );
    assert!(
        xml.contains("<tt:Height>1080</tt:Height>"),
        "Default 1080 height missing: {xml}"
    );
    assert!(xml.contains("H264"), "Default H264 encoding missing: {xml}");

    // Exactly one Profiles element.
    let count = xml.matches("<trt:Profiles ").count();
    assert_eq!(
        count, 1,
        "Expected exactly 1 trt:Profiles element, got {count}: {xml}"
    );
}

// ─── Part B: advertise only registered services ───────────────────────────────

/// A DeviceServiceHandler constructed with empty xaddrs for ptz/imaging/events must
/// omit those entries from GetServices and GetCapabilities.
#[tokio::test]
async fn device_only_omits_unregistered_services_from_get_services() {
    // media is registered, ptz/imaging/events are not (empty xaddr).
    let handler = DeviceServiceHandler::new(
        Arc::new(StubDevice),
        "http://host/onvif/device_service",
        "http://host/onvif/media_service",
        "", // ptz — not registered
        "", // imaging — not registered
        "", // events — not registered
    );
    let body = Bytes::from_static(
        b"<tds:GetServices xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetServices must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    // Device service always present.
    assert!(
        xml.contains("http://www.onvif.org/ver10/device/wsdl"),
        "device service namespace missing: {xml}"
    );
    // Media is registered.
    assert!(
        xml.contains("http://www.onvif.org/ver10/media/wsdl"),
        "media service namespace missing: {xml}"
    );

    // PTZ/imaging/events must NOT appear.
    assert!(
        !xml.contains("ptz/wsdl"),
        "ptz must not appear when xaddr is empty: {xml}"
    );
    assert!(
        !xml.contains("imaging/wsdl"),
        "imaging must not appear when xaddr is empty: {xml}"
    );
    assert!(
        !xml.contains("events/wsdl"),
        "events must not appear when xaddr is empty: {xml}"
    );
}

/// GetCapabilities must also omit Media/PTZ/Imaging/Events capability entries when
/// the corresponding xaddr is empty.
#[tokio::test]
async fn device_only_omits_unregistered_services_from_get_capabilities() {
    // Nothing registered beyond device.
    let handler = DeviceServiceHandler::new(
        Arc::new(StubDevice),
        "http://host/onvif/device_service",
        "", // media — not registered
        "", // ptz — not registered
        "", // imaging — not registered
        "", // events — not registered
    );
    let body = Bytes::from_static(
        b"<tds:GetCapabilities xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetCapabilities must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    // Device capability always present.
    assert!(
        xml.contains("<tt:Device>"),
        "tt:Device entry missing: {xml}"
    );

    // All optional services omitted.
    assert!(
        !xml.contains("<tt:Media>"),
        "tt:Media must not appear when xaddr is empty: {xml}"
    );
    assert!(
        !xml.contains("<tt:PTZ>"),
        "tt:PTZ must not appear when xaddr is empty: {xml}"
    );
    assert!(
        !xml.contains("<tt:Imaging>"),
        "tt:Imaging must not appear when xaddr is empty: {xml}"
    );
    assert!(
        !xml.contains("<tt:Events>"),
        "tt:Events must not appear when xaddr is empty: {xml}"
    );
}
