// tests/odm_smoke.rs
// ODM (ONVIF Device Manager) smoke test — Phase 5 Plan 2
// Validates the full five-service surface as an ODM client would exercise it.
// All tests call handlers directly without starting an HTTP server.

use std::sync::Arc;
use bytes::Bytes;
use soap_server::SoapHandler;
use onvif_server::{
    DeviceService, ImagingService, EventService,
    DeviceServiceHandler, ImagingServiceHandler, EventServiceHandler,
    OnvifError, ImagingSettings,
};
use onvif_server::generated::DeviceInfo;

// ---------------------------------------------------------------------------
// Test stubs
// ---------------------------------------------------------------------------

struct OdmTestDevice;

#[async_trait::async_trait]
impl DeviceService for OdmTestDevice {
    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        Ok(DeviceInfo {
            manufacturer: "TestMfr".to_string(),
            model: "TestModel".to_string(),
            firmware_version: "1.0".to_string(),
            serial_number: "SN-001".to_string(),
            hardware_id: "HW-001".to_string(),
        })
    }
}

struct OdmTestImaging;

#[async_trait::async_trait]
impl ImagingService for OdmTestImaging {
    async fn get_imaging_settings(&self, _token: String) -> Result<ImagingSettings, OnvifError> {
        Ok(ImagingSettings {
            brightness: Some(42.0),
            ..Default::default()
        })
    }
}

struct OdmTestEvent;

#[async_trait::async_trait]
impl EventService for OdmTestEvent {}

// ---------------------------------------------------------------------------
// Handler constructors (using 5-xaddr DeviceServiceHandler)
// ---------------------------------------------------------------------------

fn make_device_handler() -> DeviceServiceHandler {
    DeviceServiceHandler::new(
        Arc::new(OdmTestDevice),
        "http://0.0.0.0:8080/onvif/device_service",
        "http://0.0.0.0:8080/onvif/media_service",
        "http://0.0.0.0:8080/onvif/ptz_service",
        "http://0.0.0.0:8080/onvif/imaging_service",
        "http://0.0.0.0:8080/onvif/events_service",
    )
}

fn make_imaging_handler() -> ImagingServiceHandler {
    ImagingServiceHandler::new(Arc::new(OdmTestImaging))
}

fn make_event_handler() -> EventServiceHandler {
    EventServiceHandler::new(Arc::new(OdmTestEvent), "http://0.0.0.0:8080/onvif/events_service")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn odm_smoke_get_capabilities() {
    let handler = make_device_handler();
    let body = Bytes::from_static(
        b"<tds:GetCapabilities xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await
        .expect("GetCapabilities must not return SoapFault");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert!(
        xml.contains("imaging_service"),
        "GetCapabilities must contain imaging_service XAddr, got: {xml}"
    );
    assert!(
        xml.contains("events_service"),
        "GetCapabilities must contain events_service XAddr, got: {xml}"
    );
}

#[tokio::test]
async fn odm_smoke_get_device_information() {
    let handler = make_device_handler();
    let body = Bytes::from_static(
        b"<tds:GetDeviceInformation xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await
        .expect("GetDeviceInformation must not return SoapFault");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert!(
        xml.contains("Manufacturer"),
        "GetDeviceInformation must contain Manufacturer element, got: {xml}"
    );
    assert!(
        xml.contains("Model"),
        "GetDeviceInformation must contain Model element, got: {xml}"
    );
}

#[tokio::test]
async fn odm_smoke_get_services() {
    let handler = make_device_handler();
    let body = Bytes::from_static(
        b"<tds:GetServices xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\">\
          <tds:IncludeCapability>false</tds:IncludeCapability>\
          </tds:GetServices>",
    );
    let result = handler.handle(body).await
        .expect("GetServices must not return SoapFault");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert!(
        xml.contains("ver20/imaging"),
        "GetServices must contain imaging namespace (ver20/imaging), got: {xml}"
    );
    assert!(
        xml.contains("ver10/events"),
        "GetServices must contain events namespace (ver10/events), got: {xml}"
    );
}

#[tokio::test]
async fn odm_smoke_get_imaging_settings() {
    let handler = make_imaging_handler();
    let body = Bytes::from_static(
        b"<timg:GetImagingSettings xmlns:timg=\"http://www.onvif.org/ver20/imaging/wsdl\">\
          <timg:VideoSourceToken>VideoSourceToken</timg:VideoSourceToken>\
          </timg:GetImagingSettings>",
    );
    let result = handler.handle(body).await
        .expect("GetImagingSettings must not return SoapFault");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert!(
        xml.contains("timg:ImagingSettings"),
        "GetImagingSettings must contain timg:ImagingSettings element, got: {xml}"
    );
}

#[tokio::test]
async fn odm_smoke_event_lifecycle() {
    let handler = make_event_handler();

    // Step 1: CreatePullPointSubscription
    let create_body = Bytes::from_static(
        b"<tev:CreatePullPointSubscription xmlns:tev=\"http://www.onvif.org/ver10/events/wsdl\"/>",
    );
    let create_result = handler.handle(create_body).await
        .expect("CreatePullPointSubscription must not return SoapFault");
    let create_xml = String::from_utf8(create_result.to_vec()).unwrap();

    assert!(
        create_xml.contains("SubscriptionReference"),
        "CreatePullPointSubscription must contain SubscriptionReference, got: {create_xml}"
    );
    assert!(
        create_xml.contains("wsa5:Address"),
        "CreatePullPointSubscription must contain wsa5:Address, got: {create_xml}"
    );

    // Extract SubscriptionId from response
    let sub_id = {
        let start_tag = "<tev:SubscriptionId>";
        let end_tag = "</tev:SubscriptionId>";
        let start = create_xml.find(start_tag)
            .expect("SubscriptionId start tag not found")
            + start_tag.len();
        let end = create_xml[start..].find(end_tag)
            .expect("SubscriptionId end tag not found")
            + start;
        create_xml[start..end].to_string()
    };

    // Step 2: PullMessages
    let pull_body = format!(
        "<tev:PullMessages xmlns:tev=\"http://www.onvif.org/ver10/events/wsdl\">\
         <tev:MessageLimit>10</tev:MessageLimit>\
         <tev:SubscriptionId>{sub_id}</tev:SubscriptionId>\
         </tev:PullMessages>"
    );
    let pull_result = handler.handle(Bytes::from(pull_body)).await
        .expect("PullMessages must not return SoapFault");
    let pull_xml = String::from_utf8(pull_result.to_vec()).unwrap();

    assert!(
        pull_xml.contains("PullMessagesResponse"),
        "PullMessages must contain PullMessagesResponse, got: {pull_xml}"
    );
    assert!(
        pull_xml.contains("CurrentTime"),
        "PullMessages must contain CurrentTime, got: {pull_xml}"
    );

    // Step 3: Unsubscribe
    let unsub_body = format!(
        "<tev:Unsubscribe xmlns:tev=\"http://www.onvif.org/ver10/events/wsdl\">\
         <tev:SubscriptionId>{sub_id}</tev:SubscriptionId>\
         </tev:Unsubscribe>"
    );
    let unsub_result = handler.handle(Bytes::from(unsub_body)).await;
    assert!(
        unsub_result.is_ok(),
        "Unsubscribe must return Ok, got: {:?}", unsub_result.err()
    );
}

#[tokio::test]
async fn odm_smoke_full_sequence() {
    let device_handler = make_device_handler();
    let imaging_handler = make_imaging_handler();
    let event_handler = make_event_handler();

    // 1. GetCapabilities
    let caps = device_handler.handle(Bytes::from_static(
        b"<tds:GetCapabilities xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    )).await.expect("GetCapabilities must not fault");
    let caps_xml = String::from_utf8(caps.to_vec()).unwrap();
    assert!(caps_xml.contains("imaging_service"), "full_sequence: GetCapabilities missing imaging_service");
    assert!(caps_xml.contains("events_service"), "full_sequence: GetCapabilities missing events_service");

    // 2. GetDeviceInformation
    let devinfo = device_handler.handle(Bytes::from_static(
        b"<tds:GetDeviceInformation xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    )).await.expect("GetDeviceInformation must not fault");
    let devinfo_xml = String::from_utf8(devinfo.to_vec()).unwrap();
    assert!(devinfo_xml.contains("Manufacturer"), "full_sequence: GetDeviceInformation missing Manufacturer");

    // 3. GetServices
    let svcs = device_handler.handle(Bytes::from_static(
        b"<tds:GetServices xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\">\
          <tds:IncludeCapability>false</tds:IncludeCapability>\
          </tds:GetServices>",
    )).await.expect("GetServices must not fault");
    let svcs_xml = String::from_utf8(svcs.to_vec()).unwrap();
    assert!(svcs_xml.contains("ver20/imaging"), "full_sequence: GetServices missing imaging namespace");
    assert!(svcs_xml.contains("ver10/events"), "full_sequence: GetServices missing events namespace");

    // 4. GetImagingSettings
    let img = imaging_handler.handle(Bytes::from_static(
        b"<timg:GetImagingSettings xmlns:timg=\"http://www.onvif.org/ver20/imaging/wsdl\">\
          <timg:VideoSourceToken>VideoSourceToken</timg:VideoSourceToken>\
          </timg:GetImagingSettings>",
    )).await.expect("GetImagingSettings must not fault");
    let img_xml = String::from_utf8(img.to_vec()).unwrap();
    assert!(img_xml.contains("timg:ImagingSettings"), "full_sequence: GetImagingSettings missing timg:ImagingSettings");

    // 5. Event lifecycle — CreatePullPointSubscription
    let create = event_handler.handle(Bytes::from_static(
        b"<tev:CreatePullPointSubscription xmlns:tev=\"http://www.onvif.org/ver10/events/wsdl\"/>",
    )).await.expect("CreatePullPointSubscription must not fault");
    let create_xml = String::from_utf8(create.to_vec()).unwrap();
    let sub_id = {
        let start_tag = "<tev:SubscriptionId>";
        let end_tag = "</tev:SubscriptionId>";
        let start = create_xml.find(start_tag).expect("no SubscriptionId tag") + start_tag.len();
        let end = create_xml[start..].find(end_tag).expect("no SubscriptionId close") + start;
        create_xml[start..end].to_string()
    };

    // 6. PullMessages
    event_handler.handle(Bytes::from(format!(
        "<tev:PullMessages xmlns:tev=\"http://www.onvif.org/ver10/events/wsdl\">\
         <tev:MessageLimit>10</tev:MessageLimit>\
         <tev:SubscriptionId>{sub_id}</tev:SubscriptionId>\
         </tev:PullMessages>"
    ))).await.expect("PullMessages must not fault");

    // 7. Unsubscribe
    event_handler.handle(Bytes::from(format!(
        "<tev:Unsubscribe xmlns:tev=\"http://www.onvif.org/ver10/events/wsdl\">\
         <tev:SubscriptionId>{sub_id}</tev:SubscriptionId>\
         </tev:Unsubscribe>"
    ))).await.expect("Unsubscribe must not fault");
}
