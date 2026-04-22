use bytes::Bytes;
use onvif_server::generated::ImagingSettings;
use onvif_server::service::imaging::ImagingServiceHandler;
use onvif_server::{ImagingService, OnvifError};
use soap_server::SoapHandler;
use std::sync::Arc;

struct TestImaging;

#[async_trait::async_trait]
impl ImagingService for TestImaging {
    async fn get_imaging_settings(
        &self,
        _video_source_token: String,
    ) -> Result<ImagingSettings, OnvifError> {
        Ok(ImagingSettings {
            brightness: Some(50.0),
            contrast: Some(50.0),
            ..Default::default()
        })
    }
}

fn make_handler() -> ImagingServiceHandler {
    ImagingServiceHandler::new(Arc::new(TestImaging))
}

#[tokio::test]
async fn imaging_get_imaging_settings_response_element() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<timg:GetImagingSettings xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"><timg:VideoSourceToken>video_source_0</timg:VideoSourceToken></timg:GetImagingSettings>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetImagingSettings failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("timg:GetImagingSettingsResponse"),
        "response must contain timg:GetImagingSettingsResponse: {xml}"
    );
}

#[tokio::test]
async fn imaging_get_imaging_settings_contains_imaging_settings_element() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<timg:GetImagingSettings xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"><timg:VideoSourceToken>video_source_0</timg:VideoSourceToken></timg:GetImagingSettings>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetImagingSettings failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("timg:ImagingSettings"),
        "response must contain timg:ImagingSettings: {xml}"
    );
}

#[tokio::test]
async fn imaging_brightness_value_in_xml() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<timg:GetImagingSettings xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"><timg:VideoSourceToken>video_source_0</timg:VideoSourceToken></timg:GetImagingSettings>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetImagingSettings failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("<tt:Brightness>50</tt:Brightness>"),
        "response must contain brightness 50: {xml}"
    );
}

#[tokio::test]
async fn imaging_none_fields_produce_empty_imaging_settings() {
    struct NoneImaging;
    #[async_trait::async_trait]
    impl ImagingService for NoneImaging {
        async fn get_imaging_settings(
            &self,
            _video_source_token: String,
        ) -> Result<ImagingSettings, OnvifError> {
            Ok(ImagingSettings::default())
        }
    }
    let handler = ImagingServiceHandler::new(Arc::new(NoneImaging));
    let body = Bytes::from(
        r#"<timg:GetImagingSettings xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"><timg:VideoSourceToken>video_source_0</timg:VideoSourceToken></timg:GetImagingSettings>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetImagingSettings failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    // No stray empty child elements — no <tt:Brightness/> or similar
    assert!(
        !xml.contains("<tt:Brightness>"),
        "no Brightness element when None: {xml}"
    );
    assert!(
        !xml.contains("<tt:Contrast>"),
        "no Contrast element when None: {xml}"
    );
}

#[tokio::test]
async fn imaging_unknown_operation_returns_soap_fault() {
    let handler = make_handler();
    let body =
        Bytes::from(r#"<timg:UnknownOp xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"/>"#);
    let result = handler.handle(body).await;
    assert!(result.is_err(), "unknown operation must return SoapFault");
}
