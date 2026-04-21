use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};
use quick_xml::NsReader;
use quick_xml::events::Event;

use crate::error::OnvifError;
use crate::traits::MediaService;
use crate::constants::{
    PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN,
    TRANSLATION_SPACE_FOV, VIDEO_ENCODER_TOKEN,
};

#[allow(dead_code)]
pub struct MediaServiceHandler {
    pub(crate) svc: Arc<dyn MediaService>,
    pub(crate) xaddr: String,
}

impl MediaServiceHandler {
    pub fn new(svc: Arc<dyn MediaService>, xaddr: impl Into<String>) -> Self {
        Self { svc, xaddr: xaddr.into() }
    }
}

#[async_trait]
impl SoapHandler for MediaServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetProfiles"                   => self.handle_get_profiles().await,
            "GetStreamUri"                  => self.handle_get_stream_uri(&body).await,
            "GetSnapshotUri"                => self.handle_get_snapshot_uri(&body).await,
            "GetVideoSources"               => self.handle_get_video_sources().await,
            "GetVideoSourceConfigurations"  => self.handle_get_video_source_configurations().await,
            "GetVideoEncoderConfigurations" => self.handle_get_video_encoder_configurations().await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}

fn extract_local_name(body: &Bytes) -> Result<String, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_resolved_event().map_err(|e| SoapFault::sender(format!("{e}")))? {
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
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    let mut inside_target = false;
    loop {
        match reader.read_resolved_event().map_err(|e| SoapFault::sender(format!("{e}")))? {
            (_, Event::Start(e)) => {
                let local_name = e.local_name();
                let local = std::str::from_utf8(local_name.as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?;
                if local == element_name {
                    inside_target = true;
                }
            }
            (_, Event::Text(t)) if inside_target => {
                return std::str::from_utf8(t.as_ref())
                    .map(|s| s.to_owned())
                    .map_err(|e| SoapFault::sender(format!("{e}")));
            }
            (_, Event::Eof) => return Err(SoapFault::sender(
                format!("Element {element_name} not found in body")
            )),
            _ => {}
        }
    }
}

impl MediaServiceHandler {
    async fn handle_get_profiles(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<trt:GetProfilesResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:Profiles token="{profile_token}" fixed="true">
    <tt:Name>MainProfile</tt:Name>
    <tt:VideoSourceConfiguration token="{vs_cfg_token}">
      <tt:Name>VideoSourceConfig</tt:Name>
      <tt:UseCount>1</tt:UseCount>
      <tt:SourceToken>{vs_token}</tt:SourceToken>
      <tt:Bounds x="0" y="0" width="1920" height="1080"/>
    </tt:VideoSourceConfiguration>
    <tt:VideoEncoderConfiguration token="{ve_cfg_token}">
      <tt:Name>VideoEncoderConfig</tt:Name>
      <tt:UseCount>1</tt:UseCount>
      <tt:Encoding>H264</tt:Encoding>
      <tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution>
      <tt:Quality>5</tt:Quality>
      <tt:RateControl>
        <tt:FrameRateLimit>30</tt:FrameRateLimit>
        <tt:EncodingInterval>1</tt:EncodingInterval>
        <tt:BitrateLimit>4096</tt:BitrateLimit>
      </tt:RateControl>
      <tt:Multicast>
        <tt:Address><tt:Type>IPv4</tt:Type><tt:IPv4Address>0.0.0.0</tt:IPv4Address></tt:Address>
        <tt:Port>0</tt:Port>
        <tt:TTL>0</tt:TTL>
        <tt:AutoStart>false</tt:AutoStart>
      </tt:Multicast>
      <tt:SessionTimeout>PT10S</tt:SessionTimeout>
    </tt:VideoEncoderConfiguration>
    <tt:PTZConfiguration token="{ptz_cfg_token}">
      <tt:Name>PTZConfig</tt:Name>
      <tt:UseCount>1</tt:UseCount>
      <tt:NodeToken>{ptz_node_token}</tt:NodeToken>
      <tt:DefaultContinuousPanTiltVelocitySpace>{translation_space_fov}</tt:DefaultContinuousPanTiltVelocitySpace>
    </tt:PTZConfiguration>
  </trt:Profiles>
</trt:GetProfilesResponse>"#,
            profile_token = PROFILE_TOKEN,
            vs_cfg_token = VIDEO_SOURCE_TOKEN,
            vs_token = VIDEO_SOURCE_TOKEN,
            ve_cfg_token = VIDEO_ENCODER_TOKEN,
            ptz_cfg_token = PTZ_CONFIG_TOKEN,
            ptz_node_token = PTZ_NODE_TOKEN,
            translation_space_fov = TRANSLATION_SPACE_FOV,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_stream_uri(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let uri = self.svc.get_stream_uri(&profile_token).await
            .map_err(|e| e.into_soap_fault())?;
        let xml = format!(
            r#"<trt:GetStreamUriResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:MediaUri>
    <tt:Uri>{uri}</tt:Uri>
    <tt:InvalidAfterConnect>false</tt:InvalidAfterConnect>
    <tt:InvalidAfterReboot>false</tt:InvalidAfterReboot>
    <tt:Timeout>PT0S</tt:Timeout>
  </trt:MediaUri>
</trt:GetStreamUriResponse>"#,
            uri = uri
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_snapshot_uri(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let profile_token = extract_text_element(body, "ProfileToken")?;
        let uri = self.svc.get_snapshot_uri(&profile_token).await
            .map_err(|e| e.into_soap_fault())?;
        let xml = format!(
            r#"<trt:GetSnapshotUriResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:MediaUri>
    <tt:Uri>{uri}</tt:Uri>
    <tt:InvalidAfterConnect>false</tt:InvalidAfterConnect>
    <tt:InvalidAfterReboot>false</tt:InvalidAfterReboot>
    <tt:Timeout>PT0S</tt:Timeout>
  </trt:MediaUri>
</trt:GetSnapshotUriResponse>"#,
            uri = uri
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_video_sources(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<trt:GetVideoSourcesResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:VideoSources token="{vs_token}">
    <tt:Framerate>30</tt:Framerate>
    <tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution>
  </trt:VideoSources>
</trt:GetVideoSourcesResponse>"#,
            vs_token = VIDEO_SOURCE_TOKEN,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_video_source_configurations(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<trt:GetVideoSourceConfigurationsResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:Configurations token="{vs_token}">
    <tt:Name>VideoSourceConfig</tt:Name>
    <tt:UseCount>1</tt:UseCount>
    <tt:SourceToken>{vs_token}</tt:SourceToken>
    <tt:Bounds x="0" y="0" width="1920" height="1080"/>
  </trt:Configurations>
</trt:GetVideoSourceConfigurationsResponse>"#,
            vs_token = VIDEO_SOURCE_TOKEN,
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_video_encoder_configurations(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<trt:GetVideoEncoderConfigurationsResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <trt:Configurations token="{ve_token}">
    <tt:Name>VideoEncoderConfig</tt:Name>
    <tt:UseCount>1</tt:UseCount>
    <tt:Encoding>H264</tt:Encoding>
    <tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution>
    <tt:Quality>5</tt:Quality>
    <tt:RateControl>
      <tt:FrameRateLimit>30</tt:FrameRateLimit>
      <tt:EncodingInterval>1</tt:EncodingInterval>
      <tt:BitrateLimit>4096</tt:BitrateLimit>
    </tt:RateControl>
    <tt:Multicast>
      <tt:Address><tt:Type>IPv4</tt:Type><tt:IPv4Address>0.0.0.0</tt:IPv4Address></tt:Address>
      <tt:Port>0</tt:Port>
      <tt:TTL>0</tt:TTL>
      <tt:AutoStart>false</tt:AutoStart>
    </tt:Multicast>
    <tt:SessionTimeout>PT10S</tt:SessionTimeout>
  </trt:Configurations>
</trt:GetVideoEncoderConfigurationsResponse>"#,
            ve_token = VIDEO_ENCODER_TOKEN,
        );
        Ok(Bytes::from(xml))
    }
}
