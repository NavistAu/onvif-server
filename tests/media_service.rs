// tests/media_service.rs
// Integration tests for Phase 3: Media Service (MEDIA-01 through MEDIA-06)
// All tests start as #[ignore] stubs; #[ignore] removed in Task 3 once handler is implemented.

use std::sync::Arc;
use bytes::Bytes;
use soap_server::SoapHandler;
use onvif_server::{MediaService, MediaServiceHandler, OnvifError};

/// Minimal MediaService implementation for tests
struct TestMedia;

#[async_trait::async_trait]
impl MediaService for TestMedia {
    async fn get_stream_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("rtsp://test/{profile_token}"))
    }

    async fn get_snapshot_uri(&self, profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("http://test/{profile_token}/snapshot.jpg"))
    }
}

fn make_handler() -> MediaServiceHandler {
    let svc = Arc::new(TestMedia);
    MediaServiceHandler::new(svc, "http://localhost:8080/onvif/media_service")
}

#[tokio::test]
#[ignore]
async fn media_get_profiles() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<trt:GetProfiles xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetProfiles must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("profile_0"),
        "Response must contain PROFILE_TOKEN, got: {xml}"
    );
    assert!(
        xml.contains("PTZConfiguration"),
        "Response must contain PTZConfiguration element, got: {xml}"
    );
    assert!(
        xml.contains("DefaultContinuousPanTiltVelocitySpace"),
        "Response must contain DefaultContinuousPanTiltVelocitySpace element, got: {xml}"
    );
    assert!(
        xml.contains("http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov"),
        "Response must contain TRANSLATION_SPACE_FOV URI, got: {xml}"
    );
}

#[tokio::test]
#[ignore]
async fn media_get_profiles_ptz_config_token() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<trt:GetProfiles xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetProfiles must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("ptz_cfg_0"),
        "Response must contain PTZ_CONFIG_TOKEN 'ptz_cfg_0', got: {xml}"
    );
}

#[tokio::test]
#[ignore]
async fn media_get_stream_uri() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
  <trt:StreamSetup>
    <tt:Stream xmlns:tt="http://www.onvif.org/ver10/schema">RTP-Unicast</tt:Stream>
    <tt:Transport xmlns:tt="http://www.onvif.org/ver10/schema"><tt:Protocol>RTSP</tt:Protocol></tt:Transport>
  </trt:StreamSetup>
  <trt:ProfileToken>profile_0</trt:ProfileToken>
</trt:GetStreamUri>"#,
    );
    let result = handler.handle(body).await.expect("GetStreamUri must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("rtsp://test/profile_0"),
        "Response must contain the consumer's RTSP URI, got: {xml}"
    );
}

#[tokio::test]
#[ignore]
async fn media_get_snapshot_uri() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<trt:GetSnapshotUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
  <trt:ProfileToken>profile_0</trt:ProfileToken>
</trt:GetSnapshotUri>"#,
    );
    let result = handler.handle(body).await.expect("GetSnapshotUri must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("http://test/profile_0/snapshot.jpg"),
        "Response must contain the consumer's snapshot URL, got: {xml}"
    );
}

#[tokio::test]
#[ignore]
async fn media_get_video_sources() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<trt:GetVideoSources xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetVideoSources must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("video_src_0"),
        "Response must contain VIDEO_SOURCE_TOKEN 'video_src_0', got: {xml}"
    );
}

#[tokio::test]
#[ignore]
async fn media_get_video_source_configurations() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<trt:GetVideoSourceConfigurations xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetVideoSourceConfigurations must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("video_src_0"),
        "Response must contain VIDEO_SOURCE_TOKEN 'video_src_0', got: {xml}"
    );
}

#[tokio::test]
#[ignore]
async fn media_get_video_encoder_configurations() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<trt:GetVideoEncoderConfigurations xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetVideoEncoderConfigurations must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("H264"),
        "Response must contain H264 encoding, got: {xml}"
    );
    assert!(
        xml.contains("Multicast"),
        "Response must contain Multicast element, got: {xml}"
    );
    assert!(
        xml.contains("SessionTimeout"),
        "Response must contain SessionTimeout element, got: {xml}"
    );
}
