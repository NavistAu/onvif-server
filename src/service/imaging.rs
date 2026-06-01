use async_trait::async_trait;
use bytes::Bytes;
use quick_xml::events::Event;
use quick_xml::NsReader;
use soap_server::{SoapFault, SoapHandler};
use std::sync::Arc;

use crate::error::OnvifError;
use crate::service::xml_util::extract_text_ns;
use crate::traits::ImagingService;

/// ONVIF namespaces accepted for imaging request elements.
const ONVIF_IMAGING_NS: &[u8] = b"http://www.onvif.org/ver20/imaging/wsdl";
const ONVIF_SCHEMA_NS: &[u8] = b"http://www.onvif.org/ver10/schema";

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
    extract_text_ns(body, element_name, &[ONVIF_IMAGING_NS, ONVIF_SCHEMA_NS])
}

impl ImagingServiceHandler {
    async fn handle_get_imaging_settings(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let token = extract_text_element(body, "VideoSourceToken")?;
        let settings = self
            .svc
            .get_imaging_settings(token)
            .await
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
            inner.push_str(&format!(
                "<tt:ColorSaturation>{}</tt:ColorSaturation>",
                v as i32
            ));
        }
        // Emit a single <tt:WhiteBalance> with both gain children when either is present.
        // ONVIF schema requires one element with CrGain and CbGain as children,
        // not two separate <tt:WhiteBalance> elements (BLOCK-OS-C06 fix).
        if settings.white_balance_cr_gain.is_some() || settings.white_balance_cb_gain.is_some() {
            let cr = settings
                .white_balance_cr_gain
                .map(|v| format!("<tt:CrGain>{v}</tt:CrGain>"))
                .unwrap_or_default();
            let cb = settings
                .white_balance_cb_gain
                .map(|v| format!("<tt:CbGain>{v}</tt:CbGain>"))
                .unwrap_or_default();
            inner.push_str(&format!("<tt:WhiteBalance>{cr}{cb}</tt:WhiteBalance>"));
        }

        let xml = format!(
            r#"<timg:GetImagingSettingsResponse xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema"><timg:ImagingSettings>{inner}</timg:ImagingSettings></timg:GetImagingSettingsResponse>"#,
            inner = inner,
        );
        Ok(Bytes::from(xml))
    }
}
