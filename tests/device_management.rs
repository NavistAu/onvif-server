// tests/device_management.rs
// Integration tests for Phase 2: Device Management (DEV-01 through DEV-07)
// Wave 0: stubs compile; #[ignore] removed as each handler is implemented.

use bytes::Bytes;
use onvif_server::{
    DeviceInfo, DeviceService, DeviceServiceHandler, EventService, ImagingService, ImagingSettings,
    MediaService, NetworkInterface, PTZService,
};
use soap_server::SoapHandler;
use std::sync::Arc;

/// A minimal DeviceService implementation for tests
struct TestDevice {
    info: DeviceInfo,
}

#[async_trait::async_trait]
impl DeviceService for TestDevice {
    async fn get_device_information(&self) -> Result<DeviceInfo, onvif_server::OnvifError> {
        Ok(self.info.clone())
    }

    async fn get_network_interfaces(
        &self,
    ) -> Result<Vec<NetworkInterface>, onvif_server::OnvifError> {
        Ok(vec![NetworkInterface {
            token: "eth0".into(),
            enabled: true,
            name: "eth0".into(),
            hw_address: "00:00:00:00:00:00".into(),
            mtu: 1500,
        }])
    }
}

/// Minimal MediaService stub — all methods use default not_implemented() responses
struct TestMedia;

#[async_trait::async_trait]
impl MediaService for TestMedia {}

/// Minimal PTZService stub — all methods use default not_implemented() responses
struct TestPTZ;

#[async_trait::async_trait]
impl PTZService for TestPTZ {}

/// Minimal ImagingService stub
struct TestImaging;

#[async_trait::async_trait]
impl ImagingService for TestImaging {
    async fn get_imaging_settings(
        &self,
        _token: String,
    ) -> Result<ImagingSettings, onvif_server::OnvifError> {
        Ok(ImagingSettings {
            brightness: Some(50.0),
            ..Default::default()
        })
    }
}

/// Minimal EventService stub — uses all defaults
struct TestEvent;

#[async_trait::async_trait]
impl EventService for TestEvent {}

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
    DeviceServiceHandler::new(
        svc,
        "http://192.168.1.10:8080/onvif/device_service",
        "http://192.168.1.10:8080/onvif/media_service",
        "http://192.168.1.10:8080/onvif/ptz_service",
        "http://192.168.1.10:8080/onvif/imaging_service",
        "http://192.168.1.10:8080/onvif/events_service",
    )
}

#[tokio::test]
async fn device_get_system_date_and_time() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetSystemDateAndTime xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetSystemDateAndTime must succeed");
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
    let result = handler
        .handle(body)
        .await
        .expect("GetCapabilities must succeed");
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
    let result = handler
        .handle(body)
        .await
        .expect("GetServices must succeed");
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
    assert!(
        xml.contains("http://www.onvif.org/ver10/ptz/wsdl"),
        "GetServices must advertise PTZ as ver10/ptz/wsdl (not ver20): {xml}"
    );
    assert!(
        !xml.contains("http://www.onvif.org/ver20/ptz/wsdl"),
        "GetServices must NOT advertise PTZ as ver20/ptz/wsdl: {xml}"
    );
}

#[tokio::test]
async fn device_get_device_information() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetDeviceInformation xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetDeviceInformation must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("Acme"),
        "Manufacturer must be present, got: {xml}"
    );
    assert!(xml.contains("Cam1"), "Model must be present, got: {xml}");
    assert!(
        xml.contains("1.0.0"),
        "FirmwareVersion must be present, got: {xml}"
    );
    assert!(
        xml.contains("SN-001"),
        "SerialNumber must be present, got: {xml}"
    );
    assert!(
        xml.contains("HW-REV-A"),
        "HardwareId must be present, got: {xml}"
    );
    assert!(
        xml.contains("tds:Manufacturer"),
        "tds:Manufacturer element must be present, got: {xml}"
    );
}

#[tokio::test]
async fn device_get_scopes() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetScopes xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("GetScopes must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("onvif://www.onvif.org/type/video_encoder"),
        "Response must contain video_encoder scope URI, got: {xml}"
    );
    assert!(
        xml.contains("onvif://www.onvif.org/Profile/Streaming"),
        "Response must contain Profile/Streaming scope URI, got: {xml}"
    );
    assert!(
        xml.contains("Fixed"),
        "Response must contain Fixed scope definition, got: {xml}"
    );
}

#[tokio::test]
async fn device_get_hostname() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetHostname xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetHostname must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("<tt:Name>onvif-device</tt:Name>"),
        "Response must contain tt:Name element with default hostname, got: {xml}"
    );
    assert!(
        xml.contains("tt:FromDHCP"),
        "Response must contain tt:FromDHCP element, got: {xml}"
    );
}

#[tokio::test]
async fn device_get_network_interfaces() {
    let handler = make_handler();
    let body = Bytes::from_static(
        b"<tds:GetNetworkInterfaces xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetNetworkInterfaces must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("token=\"eth0\""),
        "Response must contain NetworkInterfaces element with token attribute, got: {xml}"
    );
    assert!(
        xml.contains("tt:Enabled"),
        "Response must contain tt:Enabled element, got: {xml}"
    );
    assert!(
        xml.contains("tt:HwAddress"),
        "Response must contain tt:HwAddress element, got: {xml}"
    );
}

/// Verify that OnvifServer::run() binds a port and GetSystemDateAndTime (auth-exempt)
/// returns a valid SOAP response over HTTP without any WS-Security header.
#[tokio::test]
async fn device_server_binds_and_serves_auth_exempt_op() {
    use onvif_server::OnvifServer;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // Pick an OS-assigned free port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        .auth("admin", "secret")
        .device_service(TestDevice {
            info: DeviceInfo {
                manufacturer: "Test".into(),
                model: "Test".into(),
                firmware_version: "0.0.1".into(),
                serial_number: "TEST-001".into(),
                hardware_id: "HW-TEST".into(),
            },
        })
        .media_service(TestMedia)
        .ptz_service(TestPTZ)
        .imaging_service(TestImaging)
        .event_service(TestEvent)
        .build()
        .expect("build must succeed");

    // Spawn server in background task
    tokio::spawn(async move {
        server.run().await.unwrap();
    });

    // Brief wait for server to bind
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Send a raw HTTP SOAP request for GetSystemDateAndTime (no auth required)
    let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetSystemDateAndTime xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let request = format!(
        "POST /onvif/device_service HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        soap_body.len(),
        soap_body
    );

    let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .expect("must connect to test server");
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let response_str = String::from_utf8_lossy(&response);

    assert!(
        response_str.starts_with("HTTP/1.1 200"),
        "GetSystemDateAndTime must return HTTP 200 without auth, got: {response_str}"
    );
    assert!(
        response_str.contains("GetSystemDateAndTimeResponse"),
        "Response body must contain GetSystemDateAndTimeResponse, got: {response_str}"
    );
}

#[tokio::test]
async fn device_auth_valid_credential() {
    use onvif_server::OnvifServer;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        .auth("admin", "secret")
        .device_service(TestDevice {
            info: DeviceInfo {
                manufacturer: "Test".into(),
                model: "Test".into(),
                firmware_version: "0.0.1".into(),
                serial_number: "TEST-001".into(),
                hardware_id: "HW-TEST".into(),
            },
        })
        .media_service(TestMedia)
        .ptz_service(TestPTZ)
        .imaging_service(TestImaging)
        .event_service(TestEvent)
        .build()
        .expect("build must succeed");

    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let created = chrono::Utc::now().to_rfc3339();
    let soap_body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
            xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
  <s:Header>
    <wsse:Security>
      <wsse:UsernameToken>
        <wsse:Username>admin</wsse:Username>
        <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText">secret</wsse:Password>
        <wsu:Created>{created}</wsu:Created>
      </wsse:UsernameToken>
    </wsse:Security>
  </s:Header>
  <s:Body>
    <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#
    );

    let request = format!(
        "POST /onvif/device_service HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        soap_body.len(), soap_body
    );

    let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let response_str = String::from_utf8_lossy(&response);

    assert!(
        response_str.starts_with("HTTP/1.1 200"),
        "Valid credentials must return HTTP 200, got: {response_str}"
    );
    assert!(
        response_str.contains("GetDeviceInformationResponse"),
        "Response body must contain GetDeviceInformationResponse, got: {response_str}"
    );
}

/// Verify PTZ GetConfigurationOptions routes correctly over HTTP with ver10 namespace.
/// This test catches the class of bug where GetServices advertises a wrong namespace,
/// causing clients to send requests the dispatch table cannot route.
#[tokio::test]
async fn ptz_dispatch_get_configuration_options_over_http() {
    use onvif_server::OnvifServer;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        .auth("admin", "secret")
        .device_service(TestDevice {
            info: DeviceInfo {
                manufacturer: "Test".into(),
                model: "Test".into(),
                firmware_version: "0.0.1".into(),
                serial_number: "TEST-001".into(),
                hardware_id: "HW-TEST".into(),
            },
        })
        .media_service(TestMedia)
        .ptz_service(TestPTZ)
        .imaging_service(TestImaging)
        .event_service(TestEvent)
        .build()
        .expect("build must succeed");

    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Use ver10 namespace — matches what GetServices advertises and what the dispatch
    // table is keyed on. A regression to ver20 in GetServices would cause this to fail.
    let created = chrono::Utc::now().to_rfc3339();
    let soap_body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
            xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
  <s:Header>
    <wsse:Security>
      <wsse:UsernameToken>
        <wsse:Username>admin</wsse:Username>
        <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText">secret</wsse:Password>
        <wsu:Created>{created}</wsu:Created>
      </wsse:UsernameToken>
    </wsse:Security>
  </s:Header>
  <s:Body>
    <tptz:GetConfigurationOptions xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
      <tptz:ConfigurationToken>ptz_cfg_0</tptz:ConfigurationToken>
    </tptz:GetConfigurationOptions>
  </s:Body>
</s:Envelope>"#
    );

    let request = format!(
        "POST /onvif/ptz_service HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        soap_body.len(),
        soap_body
    );

    let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .expect("must connect to test server");
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let response_str = String::from_utf8_lossy(&response);

    assert!(
        response_str.starts_with("HTTP/1.1 200"),
        "GetConfigurationOptions must return HTTP 200, got: {response_str}"
    );
    assert!(
        response_str.contains("TranslationSpaceFov"),
        "GetConfigurationOptions response must contain TranslationSpaceFov URI, got: {response_str}"
    );
}

#[tokio::test]
async fn device_auth_invalid_credential() {
    use onvif_server::OnvifServer;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        .auth("admin", "secret")
        .device_service(TestDevice {
            info: DeviceInfo {
                manufacturer: "Test".into(),
                model: "Test".into(),
                firmware_version: "0.0.1".into(),
                serial_number: "TEST-001".into(),
                hardware_id: "HW-TEST".into(),
            },
        })
        .media_service(TestMedia)
        .ptz_service(TestPTZ)
        .imaging_service(TestImaging)
        .event_service(TestEvent)
        .build()
        .expect("build must succeed");

    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
  <s:Header>
    <wsse:Security>
      <wsse:UsernameToken>
        <wsse:Username>admin</wsse:Username>
        <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText">wrongpassword</wsse:Password>
      </wsse:UsernameToken>
    </wsse:Security>
  </s:Header>
  <s:Body>
    <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let request = format!(
        "POST /onvif/device_service HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        soap_body.len(), soap_body
    );

    let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let response_str = String::from_utf8_lossy(&response);

    assert!(
        !response_str.starts_with("HTTP/1.1 200") || response_str.contains("Fault"),
        "Invalid credentials must not return HTTP 200 with a success body, got: {response_str}"
    );
}
