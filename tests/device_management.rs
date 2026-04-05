// tests/device_management.rs
// Integration tests for Phase 2: Device Management (DEV-01 through DEV-07)
// Wave 0: stubs compile; #[ignore] removed as each handler is implemented.

use std::sync::Arc;
use bytes::Bytes;
use soap_server::SoapHandler;
use onvif_server::{DeviceService, DeviceServiceHandler, DeviceInfo};

/// A minimal DeviceService implementation for tests
struct TestDevice {
    info: DeviceInfo,
}

#[async_trait::async_trait]
impl DeviceService for TestDevice {
    async fn get_device_information(&self) -> Result<DeviceInfo, onvif_server::OnvifError> {
        Ok(self.info.clone())
    }
}

fn make_handler() -> DeviceServiceHandler {
    let svc = Arc::new(TestDevice {
        info: DeviceInfo {
            manufacturer: "Acme".into(),
            model: "Cam1".into(),
            firmware_version: "1.0.0".into(),
            serial_number: "SN-001".into(),
            hardware_id: "HW-REV-A".into(),
        },
    });
    DeviceServiceHandler::new(svc, "http://192.168.1.10:8080/onvif/device_service")
}

#[tokio::test]
async fn device_get_system_date_and_time() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetSystemDateAndTime xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetSystemDateAndTime must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("tt:UTCDateTime"),
        "Response must contain tt:UTCDateTime, got: {xml}"
    );
    assert!(
        xml.contains("tt:Hour"),
        "Response must contain tt:Hour, got: {xml}"
    );
    assert!(
        xml.contains("tt:Year"),
        "Response must contain tt:Year, got: {xml}"
    );
    assert!(
        xml.contains(r#"xmlns:tds="http://www.onvif.org/ver10/device/wsdl""#),
        "Response must declare tds namespace, got: {xml}"
    );
    assert!(
        xml.contains(r#"xmlns:tt="http://www.onvif.org/ver10/schema""#),
        "Response must declare tt namespace, got: {xml}"
    );
}

#[tokio::test]
async fn device_get_capabilities_xaddr() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetCapabilities xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetCapabilities must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("tt:XAddr"),
        "Response must contain tt:XAddr, got: {xml}"
    );
    assert!(
        xml.contains("http://192.168.1.10:8080/onvif/device_service"),
        "Response must contain the handler's xaddr, got: {xml}"
    );
}

#[tokio::test]
async fn device_get_services() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetServices xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetServices must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("tds:Service"),
        "Response must contain tds:Service element, got: {xml}"
    );
    assert!(
        xml.contains("tds:Namespace"),
        "Response must contain tds:Namespace element, got: {xml}"
    );
    assert!(
        xml.contains("http://192.168.1.10:8080/onvif/device_service"),
        "Response must contain the handler's xaddr in tds:XAddr, got: {xml}"
    );
}

#[tokio::test]
async fn device_get_device_information() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetDeviceInformation xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetDeviceInformation must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(xml.contains("Acme"), "Manufacturer must be present, got: {xml}");
    assert!(xml.contains("Cam1"), "Model must be present, got: {xml}");
    assert!(xml.contains("1.0.0"), "FirmwareVersion must be present, got: {xml}");
    assert!(xml.contains("SN-001"), "SerialNumber must be present, got: {xml}");
    assert!(xml.contains("HW-REV-A"), "HardwareId must be present, got: {xml}");
    assert!(xml.contains("tds:Manufacturer"), "tds:Manufacturer element must be present, got: {xml}");
}

#[tokio::test]
#[ignore]
async fn device_get_scopes() {
    // TODO: authenticated GetScopes; assert Fixed scope URIs present
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_hostname() {
    // TODO: authenticated GetHostname; assert HostnameInformation/Name present
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_network_interfaces() {
    // TODO: authenticated GetNetworkInterfaces; assert at least one NetworkInterfaces element
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_auth_valid_credential() {
    // TODO: valid WS-Security UsernameToken digest → HTTP 200 on authenticated operation
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_auth_invalid_credential() {
    // TODO: wrong password → SOAP auth fault response (not HTTP 200)
    todo!()
}
