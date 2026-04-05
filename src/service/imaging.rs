use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};
use quick_xml::NsReader;
use quick_xml::events::Event;

use crate::error::OnvifError;
use crate::traits::ImagingService;

pub struct ImagingServiceHandler {
    pub(crate) svc: Arc<dyn ImagingService>,
}

impl ImagingServiceHandler {
    pub fn new(svc: Arc<dyn ImagingService>) -> Self {
        Self { svc }
    }
}

#[async_trait]
impl SoapHandler for ImagingServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetImagingSettings" => self.handle_get_imaging_settings(&body).await,
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

impl ImagingServiceHandler {
    async fn handle_get_imaging_settings(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let token = extract_text_element(body, "VideoSourceToken")?;
        let settings = self.svc.get_imaging_settings(token).await
            .map_err(|e| e.into_soap_fault())?;

        let mut inner = String::new();
        if let Some(v) = settings.brightness {
            inner.push_str(&format!("<tt:Brightness>{}</tt:Brightness>", v as i32));
        }
        if let Some(v) = settings.contrast {
            inner.push_str(&format!("<tt:Contrast>{}</tt:Contrast>", v as i32));
        }
        if let Some(v) = settings.sharpness {
            inner.push_str(&format!("<tt:Sharpness>{}</tt:Sharpness>", v as i32));
        }
        if let Some(v) = settings.color_saturation {
            inner.push_str(&format!("<tt:ColorSaturation>{}</tt:ColorSaturation>", v as i32));
        }
        if let Some(v) = settings.white_balance_cr_gain {
            inner.push_str(&format!("<tt:WhiteBalance><tt:CrGain>{}</tt:CrGain></tt:WhiteBalance>", v));
        }
        if let Some(v) = settings.white_balance_cb_gain {
            inner.push_str(&format!("<tt:WhiteBalance><tt:CbGain>{}</tt:CbGain></tt:WhiteBalance>", v));
        }

        let xml = format!(
            r#"<timg:GetImagingSettingsResponse xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema"><timg:ImagingSettings>{inner}</timg:ImagingSettings></timg:GetImagingSettingsResponse>"#,
            inner = inner,
        );
        Ok(Bytes::from(xml))
    }
}
