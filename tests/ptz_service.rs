use bytes::Bytes;
use onvif_server::{OnvifError, PTZPreset, PTZService, PTZServiceHandler, PTZStatusResult};
use soap_server::SoapHandler;
use std::sync::Arc;

struct TestPTZ;

#[async_trait::async_trait]
impl PTZService for TestPTZ {
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

    async fn get_status(&self, _profile_token: &str) -> Result<PTZStatusResult, OnvifError> {
        Ok(PTZStatusResult {
            pan_tilt_moving: false,
            zoom_moving: false,
        })
    }

    async fn get_presets(&self, _profile_token: &str) -> Result<Vec<PTZPreset>, OnvifError> {
        Ok(vec![PTZPreset {
            token: "preset_1".into(),
            name: "Home".into(),
        }])
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
        Ok("preset_1".into())
    }

    async fn remove_preset(
        &self,
        _profile_token: &str,
        _preset_token: &str,
    ) -> Result<(), OnvifError> {
        Ok(())
    }
}

fn make_handler() -> PTZServiceHandler {
    PTZServiceHandler::new(Arc::new(TestPTZ))
}

#[tokio::test]
async fn ptz_get_nodes() {
    let handler = make_handler();
    let body = Bytes::from(r#"<tptz:GetNodes xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>"#);
    let result = handler.handle(body).await.expect("GetNodes failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("TranslationSpaceFov"),
        "response must contain TranslationSpaceFov URI: {xml}"
    );
    assert!(
        xml.contains("ptz_node_0"),
        "response must contain PTZ_NODE_TOKEN: {xml}"
    );
}

#[tokio::test]
async fn ptz_get_service_capabilities() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:GetServiceCapabilities xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetServiceCapabilities failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    // MoveStatus MUST be an XML attribute (not a child element) — Frigate checks it via zeep attribute access
    assert!(
        xml.contains(r#"MoveStatus="true""#),
        "response must contain MoveStatus attribute: {xml}"
    );
}

#[tokio::test]
async fn ptz_get_configuration_options() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:GetConfigurationOptions xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"><tptz:ConfigurationToken>ptz_cfg_0</tptz:ConfigurationToken></tptz:GetConfigurationOptions>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetConfigurationOptions failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("TranslationSpaceFov"),
        "Spaces must contain TranslationSpaceFov URI: {xml}"
    );
}

#[tokio::test]
async fn ptz_get_configurations() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:GetConfigurations xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetConfigurations failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("ptz_cfg_0"),
        "response must contain PTZ_CONFIG_TOKEN: {xml}"
    );
    assert!(
        xml.contains("ptz_node_0"),
        "response must contain PTZ_NODE_TOKEN: {xml}"
    );
}

#[tokio::test]
async fn ptz_get_status() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:GetStatus xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"><tptz:ProfileToken>profile_0</tptz:ProfileToken></tptz:GetStatus>"#,
    );
    let result = handler.handle(body).await.expect("GetStatus failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    // Nested structure: <tt:MoveStatus><tt:PanTilt>IDLE</tt:PanTilt><tt:Zoom>IDLE</tt:Zoom></tt:MoveStatus>
    assert!(
        xml.contains("<tt:PanTilt>IDLE</tt:PanTilt>"),
        "GetStatus must have nested PanTilt IDLE: {xml}"
    );
    assert!(
        xml.contains("<tt:Zoom>IDLE</tt:Zoom>"),
        "GetStatus must have nested Zoom IDLE: {xml}"
    );
}

#[tokio::test]
async fn ptz_relative_move() {
    let handler = make_handler();
    // PanTilt x/y are XML attributes — this is the Frigate pitfall
    let body = Bytes::from(
        r#"<tptz:RelativeMove xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema"><tptz:ProfileToken>profile_0</tptz:ProfileToken><tptz:Translation><tt:PanTilt x="0.5" y="-0.3" space="http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov"/><tt:Zoom x="0.0"/></tptz:Translation></tptz:RelativeMove>"#,
    );
    let result = handler.handle(body).await.expect("RelativeMove failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("RelativeMoveResponse"),
        "response must contain RelativeMoveResponse: {xml}"
    );
}

#[tokio::test]
async fn ptz_get_presets() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:GetPresets xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"><tptz:ProfileToken>profile_0</tptz:ProfileToken></tptz:GetPresets>"#,
    );
    let result = handler.handle(body).await.expect("GetPresets failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("preset_1"),
        "response must contain preset token: {xml}"
    );
    assert!(
        xml.contains("Home"),
        "response must contain preset name: {xml}"
    );
}

#[tokio::test]
async fn ptz_goto_preset() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:GotoPreset xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"><tptz:ProfileToken>profile_0</tptz:ProfileToken><tptz:PresetToken>preset_1</tptz:PresetToken></tptz:GotoPreset>"#,
    );
    let result = handler.handle(body).await.expect("GotoPreset failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("GotoPresetResponse"),
        "response must contain GotoPresetResponse: {xml}"
    );
}

#[tokio::test]
async fn ptz_set_preset() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:SetPreset xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"><tptz:ProfileToken>profile_0</tptz:ProfileToken><tptz:PresetName>Home</tptz:PresetName></tptz:SetPreset>"#,
    );
    let result = handler.handle(body).await.expect("SetPreset failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("PresetToken"),
        "response must contain PresetToken: {xml}"
    );
}

#[tokio::test]
async fn ptz_remove_preset() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tptz:RemovePreset xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"><tptz:ProfileToken>profile_0</tptz:ProfileToken><tptz:PresetToken>preset_1</tptz:PresetToken></tptz:RemovePreset>"#,
    );
    let result = handler.handle(body).await.expect("RemovePreset failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("RemovePresetResponse"),
        "response must contain RemovePresetResponse: {xml}"
    );
}
