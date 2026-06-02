use async_trait::async_trait;
use bytes::Bytes;
use chrono::{SecondsFormat, Utc};
use quick_xml::events::Event;
use quick_xml::NsReader;
use soap_server::{escape_attr, escape_text, SoapFault, SoapHandler};
use std::sync::Arc;

use crate::constants::{PTZ_CONFIG_TOKEN, PTZ_NODE_TOKEN, TRANSLATION_SPACE_FOV};
use crate::error::OnvifError;
use crate::generated::{PTZPreset, PTZStatusResult};
use crate::service::xml_util::extract_text_ns;
use crate::traits::PTZService;

/// ONVIF namespaces accepted for PTZ request elements. Both the ver20 (current)
/// and ver10 (legacy) PTZ WSDL namespaces are accepted, since real clients use
/// either; the common schema namespace covers shared element types.
const ONVIF_PTZ_NS_V20: &[u8] = b"http://www.onvif.org/ver20/ptz/wsdl";
const ONVIF_PTZ_NS_V10: &[u8] = b"http://www.onvif.org/ver10/ptz/wsdl";
const ONVIF_SCHEMA_NS: &[u8] = b"http://www.onvif.org/ver10/schema";

/// Parse a PTZ coordinate attribute value as `f32`.
///
/// If the attribute is absent (`None`), returns `0.0` as the ONVIF default.
/// If the attribute is present but not a valid float, returns a SOAP `Sender`
/// fault with `OnvifError::InvalidArgument` — preventing silent coercion that
/// could trigger unintended physical camera movement (BLOCK-OS-C02).
fn parse_ptz_coord(val: Option<String>, field: &str) -> Result<f32, SoapFault> {
    match val {
        None => Ok(0.0),
        Some(s) => s.parse::<f32>().map_err(|_| {
            OnvifError::InvalidArgument(format!("malformed PTZ coordinate: {field}=\"{s}\""))
                .into_soap_fault()
        }),
    }
}

pub struct PTZServiceHandler {
    pub(crate) svc: Arc<dyn PTZService>,
}

impl PTZServiceHandler {
    pub fn new(svc: Arc<dyn PTZService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl SoapHandler for PTZServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetNodes" => self.handle_get_nodes().await,
            "GetNode" => self.handle_get_node(&body).await,
            "GetConfigurations" => self.handle_get_configurations().await,
            "GetConfiguration" => self.handle_get_configuration(&body).await,
            "GetConfigurationOptions" => self.handle_get_configuration_options(&body).await,
            "GetServiceCapabilities" => self.handle_get_service_capabilities().await,
            "RelativeMove" => self.handle_relative_move(&body).await,
            "AbsoluteMove" => self.handle_absolute_move(&body).await,
            "ContinuousMove" => self.handle_continuous_move(&body).await,
            "Stop" => self.handle_stop(&body).await,
            "GetStatus" => self.handle_get_status(&body).await,
            "GetPresets" => self.handle_get_presets(&body).await,
            "GotoPreset" => self.handle_goto_preset(&body).await,
            "SetPreset" => self.handle_set_preset(&body).await,
            "RemovePreset" => self.handle_remove_preset(&body).await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}

fn extract_local_name(body: &Bytes) -> Result<String, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    loop {
        match reader
            .read_resolved_event()
            .map_err(|e| SoapFault::sender(format!("{e}")))?
        {
            (_, Event::Start(e)) | (_, Event::Empty(e)) => {
                let local = std::str::from_utf8(e.local_name().as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?
                    .to_string();
                return Ok(local);
            }
            (_, Event::Eof) => return Err(SoapFault::sender("Empty body".to_string())),
            _ => {}
        }
    }
}

fn extract_text_element(body: &Bytes, element_name: &str) -> Result<String, SoapFault> {
    extract_text_ns(
        body,
        element_name,
        &[ONVIF_PTZ_NS_V20, ONVIF_PTZ_NS_V10, ONVIF_SCHEMA_NS],
    )
}

/// Find the first occurrence of `element_name` in the body and return the value of
/// the named attribute. Returns `Ok(None)` if the element is not found (caller defaults to 0.0).
fn extract_element_attribute(
    body: &Bytes,
    element_name: &str,
    attr_name: &str,
) -> Result<Option<String>, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    loop {
        match reader
            .read_resolved_event()
            .map_err(|e| SoapFault::sender(format!("{e}")))?
        {
            (_, Event::Start(e)) | (_, Event::Empty(e)) => {
                let local_bytes = e.local_name();
                let local = std::str::from_utf8(local_bytes.as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?;
                if local == element_name {
                    for attr in e.attributes() {
                        let attr = attr.map_err(|e| SoapFault::sender(format!("{e}")))?;
                        let key_bytes = attr.key.local_name();
                        let key = std::str::from_utf8(key_bytes.as_ref())
                            .map_err(|e| SoapFault::sender(format!("{e}")))?;
                        if key == attr_name {
                            let val = std::str::from_utf8(attr.value.as_ref())
                                .map_err(|e| SoapFault::sender(format!("{e}")))?
                                .to_string();
                            return Ok(Some(val));
                        }
                    }
                    // Element found but attribute absent
                    return Ok(None);
                }
            }
            (_, Event::Eof) => return Ok(None),
            _ => {}
        }
    }
}

impl PTZServiceHandler {
    // ── Discovery operations (handler-internal static XML) ─────────────────────

    async fn handle_get_nodes(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<tptz:GetNodesResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZNode token="{node_token}" FixedHomePosition="false">
    <tt:Name>PTZNode</tt:Name>
    <tt:SupportedPTZSpaces>
      <tt:RelativePanTiltTranslationSpace>
        <tt:URI>{fov_uri}</tt:URI>
        <tt:XRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:XRange>
        <tt:YRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:YRange>
      </tt:RelativePanTiltTranslationSpace>
    </tt:SupportedPTZSpaces>
    <tt:MaximumNumberOfPresets>10</tt:MaximumNumberOfPresets>
    <tt:HomeSupported>false</tt:HomeSupported>
  </tptz:PTZNode>
</tptz:GetNodesResponse>"#,
            node_token = PTZ_NODE_TOKEN,
            fov_uri = TRANSLATION_SPACE_FOV,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_node(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let token = extract_text_element(body, "NodeToken")?;
        if token != PTZ_NODE_TOKEN {
            return Err(
                OnvifError::InvalidArgument(format!("Unknown NodeToken: {token}"))
                    .into_soap_fault(),
            );
        }
        // Return same structure as GetNodes but wrapped in GetNodeResponse
        let xml = format!(
            r#"<tptz:GetNodeResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZNode token="{node_token}" FixedHomePosition="false">
    <tt:Name>PTZNode</tt:Name>
    <tt:SupportedPTZSpaces>
      <tt:RelativePanTiltTranslationSpace>
        <tt:URI>{fov_uri}</tt:URI>
        <tt:XRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:XRange>
        <tt:YRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:YRange>
      </tt:RelativePanTiltTranslationSpace>
    </tt:SupportedPTZSpaces>
    <tt:MaximumNumberOfPresets>10</tt:MaximumNumberOfPresets>
    <tt:HomeSupported>false</tt:HomeSupported>
  </tptz:PTZNode>
</tptz:GetNodeResponse>"#,
            node_token = PTZ_NODE_TOKEN,
            fov_uri = TRANSLATION_SPACE_FOV,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_configurations(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<tptz:GetConfigurationsResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZConfiguration token="{cfg_token}">
    <tt:Name>PTZConfig</tt:Name>
    <tt:UseCount>1</tt:UseCount>
    <tt:NodeToken>{node_token}</tt:NodeToken>
    <tt:DefaultContinuousPanTiltVelocitySpace>{fov_uri}</tt:DefaultContinuousPanTiltVelocitySpace>
  </tptz:PTZConfiguration>
</tptz:GetConfigurationsResponse>"#,
            cfg_token = PTZ_CONFIG_TOKEN,
            node_token = PTZ_NODE_TOKEN,
            fov_uri = TRANSLATION_SPACE_FOV,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_configuration(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let token = extract_text_element(body, "ConfigurationToken")?;
        if token != PTZ_CONFIG_TOKEN {
            return Err(OnvifError::InvalidArgument(format!(
                "Unknown ConfigurationToken: {token}"
            ))
            .into_soap_fault());
        }
        let xml = format!(
            r#"<tptz:GetConfigurationResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZConfiguration token="{cfg_token}">
    <tt:Name>PTZConfig</tt:Name>
    <tt:UseCount>1</tt:UseCount>
    <tt:NodeToken>{node_token}</tt:NodeToken>
    <tt:DefaultContinuousPanTiltVelocitySpace>{fov_uri}</tt:DefaultContinuousPanTiltVelocitySpace>
  </tptz:PTZConfiguration>
</tptz:GetConfigurationResponse>"#,
            cfg_token = PTZ_CONFIG_TOKEN,
            node_token = PTZ_NODE_TOKEN,
            fov_uri = TRANSLATION_SPACE_FOV,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_configuration_options(&self, _body: &Bytes) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<tptz:GetConfigurationOptionsResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZConfigurationOptions>
    <tt:Spaces>
      <tt:RelativePanTiltTranslationSpace>
        <tt:URI>{fov_uri}</tt:URI>
        <tt:XRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:XRange>
        <tt:YRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:YRange>
      </tt:RelativePanTiltTranslationSpace>
    </tt:Spaces>
    <tt:PTZTimeout><tt:Min>PT0S</tt:Min><tt:Max>PT60S</tt:Max></tt:PTZTimeout>
  </tptz:PTZConfigurationOptions>
</tptz:GetConfigurationOptionsResponse>"#,
            fov_uri = TRANSLATION_SPACE_FOV,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_service_capabilities(&self) -> Result<Bytes, SoapFault> {
        // MoveStatus MUST be an XML attribute on Capabilities, NOT a child element.
        // Frigate calls find_by_key(vars(capabilities), "MoveStatus") — requires attribute form.
        let xml = r#"<tptz:GetServiceCapabilitiesResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
  <tptz:Capabilities MoveStatus="true" StatusPosition="false"/>
</tptz:GetServiceCapabilitiesResponse>"#;
        Ok(Bytes::from(xml))
    }

    // ── Control operations (trait-delegated) ───────────────────────────────────

    async fn handle_get_status(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let status: PTZStatusResult = self
            .svc
            .get_status(&profile_token)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let pan_tilt = if status.pan_tilt_moving {
            "MOVING"
        } else {
            "IDLE"
        };
        let zoom = if status.zoom_moving { "MOVING" } else { "IDLE" };
        let utc_time = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let xml = format!(
            r#"<tptz:GetStatusResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZStatus>
    <tt:MoveStatus>
      <tt:PanTilt>{pan_tilt}</tt:PanTilt>
      <tt:Zoom>{zoom}</tt:Zoom>
    </tt:MoveStatus>
    <tt:UtcTime>{utc_time}</tt:UtcTime>
  </tptz:PTZStatus>
</tptz:GetStatusResponse>"#,
            pan_tilt = pan_tilt,
            zoom = zoom,
            utc_time = utc_time,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_relative_move(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let pan = parse_ptz_coord(
            extract_element_attribute(body, "PanTilt", "x")?,
            "PanTilt.x",
        )?;
        let tilt = parse_ptz_coord(
            extract_element_attribute(body, "PanTilt", "y")?,
            "PanTilt.y",
        )?;
        let zoom = parse_ptz_coord(extract_element_attribute(body, "Zoom", "x")?, "Zoom.x")?;
        self.svc
            .relative_move(&profile_token, pan, tilt, zoom)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let xml =
            r#"<tptz:RelativeMoveResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }

    async fn handle_absolute_move(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let pan = parse_ptz_coord(
            extract_element_attribute(body, "PanTilt", "x")?,
            "PanTilt.x",
        )?;
        let tilt = parse_ptz_coord(
            extract_element_attribute(body, "PanTilt", "y")?,
            "PanTilt.y",
        )?;
        let zoom = parse_ptz_coord(extract_element_attribute(body, "Zoom", "x")?, "Zoom.x")?;
        self.svc
            .absolute_move(&profile_token, pan, tilt, zoom)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let xml =
            r#"<tptz:AbsoluteMoveResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }

    async fn handle_continuous_move(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let pan = parse_ptz_coord(
            extract_element_attribute(body, "PanTilt", "x")?,
            "PanTilt.x",
        )?;
        let tilt = parse_ptz_coord(
            extract_element_attribute(body, "PanTilt", "y")?,
            "PanTilt.y",
        )?;
        let zoom = parse_ptz_coord(extract_element_attribute(body, "Zoom", "x")?, "Zoom.x")?;
        self.svc
            .continuous_move(&profile_token, pan, tilt, zoom)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let xml =
            r#"<tptz:ContinuousMoveResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }

    async fn handle_stop(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        // PanTilt and Zoom are optional booleans — absent means true (stop both)
        let pan_tilt = match extract_text_element(body, "PanTilt") {
            Ok(v) => v == "true" || v == "1",
            Err(_) => true,
        };
        let zoom = match extract_text_element(body, "Zoom") {
            Ok(v) => v == "true" || v == "1",
            Err(_) => true,
        };
        self.svc
            .stop(&profile_token, pan_tilt, zoom)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let xml = r#"<tptz:StopResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }

    async fn handle_get_presets(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let presets: Vec<PTZPreset> = self
            .svc
            .get_presets(&profile_token)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let mut preset_xml = String::new();
        for p in &presets {
            preset_xml.push_str(&format!(
                r#"  <tptz:Preset token="{token}"><tt:Name>{name}</tt:Name></tptz:Preset>
"#,
                token = escape_attr(&p.token),
                name = escape_text(&p.name),
            ));
        }
        let xml = format!(
            r#"<tptz:GetPresetsResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
{presets}</tptz:GetPresetsResponse>"#,
            presets = preset_xml,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_goto_preset(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let preset_token = extract_text_element(body, "PresetToken")?;
        self.svc
            .goto_preset(&profile_token, &preset_token)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let xml = r#"<tptz:GotoPresetResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }

    async fn handle_set_preset(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let preset_name = extract_text_element(body, "PresetName").ok();
        let preset_token = extract_text_element(body, "PresetToken").ok();
        let token = self
            .svc
            .set_preset(
                &profile_token,
                preset_name.as_deref(),
                preset_token.as_deref(),
            )
            .await
            .map_err(|e| e.into_soap_fault())?;
        let xml = format!(
            r#"<tptz:SetPresetResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
  <tptz:PresetToken>{token}</tptz:PresetToken>
</tptz:SetPresetResponse>"#,
            token = escape_text(&token),
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_remove_preset(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let preset_token = extract_text_element(body, "PresetToken")?;
        self.svc
            .remove_preset(&profile_token, &preset_token)
            .await
            .map_err(|e| e.into_soap_fault())?;
        let xml =
            r#"<tptz:RemovePresetResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }
}
