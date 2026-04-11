// tests/frigate_compat.rs
// Frigate autotracker call sequence integration test — Phase 4 Plan 2
// Validates the exact sequence Frigate PTZ autotracker uses on startup,
// exercising both MediaServiceHandler and PTZServiceHandler without HTTP.

use std::sync::Arc;
use bytes::Bytes;
use soap_server::SoapHandler;
use onvif_server::{
    PTZService, MediaService, PTZServiceHandler, MediaServiceHandler,
    OnvifError, PTZStatusResult, PTZPreset,
};

// ---------------------------------------------------------------------------
// Test stubs
// ---------------------------------------------------------------------------

struct TestMediaFrigate;

#[async_trait::async_trait]
impl MediaService for TestMediaFrigate {
    async fn get_stream_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok("rtsp://localhost/cam".into())
    }

    async fn get_snapshot_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok("http://localhost/snap.jpg".into())
    }
}

struct TestPTZFrigate;

#[async_trait::async_trait]
impl PTZService for TestPTZFrigate {
    async fn relative_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn absolute_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn continuous_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn stop(
        &self,
        _profile_token: &str,
        _pan_tilt: bool,
        _zoom: bool,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn get_status(
        &self,
        _profile_token: &str,
    ) -> Result<PTZStatusResult, OnvifError> {
        Ok(PTZStatusResult {
            pan_tilt_moving: false,
            zoom_moving: false,
        })
    }

    async fn get_presets(
        &self,
        _profile_token: &str,
    ) -> Result<Vec<PTZPreset>, OnvifError> {
        Ok(vec![])
    }

    async fn goto_preset(
        &self,
        _profile_token: &str,
        _preset_token: &str,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn set_preset(
        &self,
        _profile_token: &str,
        _preset_name: Option<&str>,
        _preset_token: Option<&str>,
    ) -> Result<String, OnvifError> {
        Ok("home".into())
    }

    async fn remove_preset(
        &self,
        _profile_token: &str,
        _preset_token: &str,
    ) -> Result<(), OnvifError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Frigate autotracker call sequence (Frigate Pattern 9 from research)
// ---------------------------------------------------------------------------

/// Exercises the exact call sequence Frigate autotracker uses on startup.
///
/// Steps:
///   1. Media:  GetProfiles → asserts PTZConfiguration tokens (ptz_cfg_0, ptz_node_0)
///   2. PTZ:    GetConfigurationOptions(ConfigurationToken=ptz_cfg_0) → TranslationSpaceFov
///   3. PTZ:    GetServiceCapabilities → MoveStatus="true" as XML attribute
///   4. PTZ:    GetPresets(ProfileToken=profile_0) → Ok
///   5. PTZ:    GetStatus(ProfileToken=profile_0) → IDLE, PanTilt, Zoom elements present
///   6. PTZ:    RelativeMove(PanTilt x="0.5" y="0.3") → Ok
///   7. PTZ:    GotoPreset(PresetToken="home") → Ok
#[tokio::test]
async fn frigate_autotracker_call_sequence() {
    let media_svc = Arc::new(TestMediaFrigate);
    let media_handler = MediaServiceHandler::new(
        media_svc,
        "http://localhost:8080/onvif/media_service",
    );

    let ptz_svc = Arc::new(TestPTZFrigate);
    let ptz_handler = PTZServiceHandler::new(ptz_svc);

    // Step 1: Media::GetProfiles — must expose ptz_cfg_0 and ptz_node_0
    {
        let body = Bytes::from_static(
            b"<trt:GetProfiles xmlns:trt=\"http://www.onvif.org/ver10/media/wsdl\"/>",
        );
        let result = media_handler.handle(body).await
            .expect("Step 1 GetProfiles must not return SoapFault");
        let xml = String::from_utf8(result.to_vec()).unwrap();

        assert!(
            xml.contains("ptz_cfg_0"),
            "Step 1: GetProfiles must contain PTZConfiguration token ptz_cfg_0 for Frigate, got: {xml}"
        );
        assert!(
            xml.contains("ptz_node_0"),
            "Step 1: GetProfiles must contain NodeToken ptz_node_0 for Frigate, got: {xml}"
        );
    }

    // Step 2: PTZ::GetConfigurationOptions — must contain TranslationSpaceFov URI
    {
        let body = Bytes::from_static(
            b"<tptz:GetConfigurationOptions \
              xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\
              <tptz:ConfigurationToken>ptz_cfg_0</tptz:ConfigurationToken>\
              </tptz:GetConfigurationOptions>",
        );
        let result = ptz_handler.handle(body).await
            .expect("Step 2 GetConfigurationOptions must not return SoapFault");
        let xml = String::from_utf8(result.to_vec()).unwrap();

        assert!(
            xml.contains("TranslationSpaceFov"),
            "Step 2: GetConfigurationOptions must contain TranslationSpaceFov URI for Frigate autotracker, got: {xml}"
        );
    }

    // Step 3: PTZ::GetServiceCapabilities — must contain MoveStatus="true" as attribute
    {
        let body = Bytes::from_static(
            b"<tptz:GetServiceCapabilities \
              xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\"/>",
        );
        let result = ptz_handler.handle(body).await
            .expect("Step 3 GetServiceCapabilities must not return SoapFault");
        let xml = String::from_utf8(result.to_vec()).unwrap();

        assert!(
            xml.contains("MoveStatus=\"true\""),
            "Step 3: GetServiceCapabilities must contain MoveStatus=\"true\" as XML attribute (not element), got: {xml}"
        );
    }

    // Step 4: PTZ::GetPresets — must return Ok (even if empty)
    {
        let body = Bytes::from_static(
            b"<tptz:GetPresets xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\
              <tptz:ProfileToken>profile_0</tptz:ProfileToken>\
              </tptz:GetPresets>",
        );
        let result = ptz_handler.handle(body).await;
        assert!(
            result.is_ok(),
            "Step 4: GetPresets must return Ok, got: {:?}", result.err()
        );
    }

    // Step 5: PTZ::GetStatus — must contain IDLE, PanTilt and Zoom elements
    {
        let body = Bytes::from_static(
            b"<tptz:GetStatus xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\
              <tptz:ProfileToken>profile_0</tptz:ProfileToken>\
              </tptz:GetStatus>",
        );
        let result = ptz_handler.handle(body).await
            .expect("Step 5 GetStatus must not return SoapFault");
        let xml = String::from_utf8(result.to_vec()).unwrap();

        assert!(
            xml.contains("IDLE"),
            "Step 5: GetStatus must contain IDLE move status, got: {xml}"
        );
        assert!(
            xml.contains("PanTilt"),
            "Step 5: GetStatus must contain PanTilt element for Frigate, got: {xml}"
        );
        assert!(
            xml.contains("Zoom"),
            "Step 5: GetStatus must contain Zoom element for Frigate, got: {xml}"
        );
    }

    // Step 6: PTZ::RelativeMove — must return Ok
    {
        let body = Bytes::from_static(
            b"<tptz:RelativeMove xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\
              <tptz:ProfileToken>profile_0</tptz:ProfileToken>\
              <tptz:Translation>\
                <tt:PanTilt xmlns:tt=\"http://www.onvif.org/ver10/schema\" x=\"0.5\" y=\"0.3\"/>\
              </tptz:Translation>\
              </tptz:RelativeMove>",
        );
        let result = ptz_handler.handle(body).await;
        assert!(
            result.is_ok(),
            "Step 6: RelativeMove must return Ok, got: {:?}", result.err()
        );
    }

    // Step 7: PTZ::GotoPreset — must return Ok
    {
        let body = Bytes::from_static(
            b"<tptz:GotoPreset xmlns:tptz=\"http://www.onvif.org/ver20/ptz/wsdl\">\
              <tptz:ProfileToken>profile_0</tptz:ProfileToken>\
              <tptz:PresetToken>home</tptz:PresetToken>\
              </tptz:GotoPreset>",
        );
        let result = ptz_handler.handle(body).await;
        assert!(
            result.is_ok(),
            "Step 7: GotoPreset must return Ok (Frigate issues GotoPreset on startup), got: {:?}", result.err()
        );
    }
}
