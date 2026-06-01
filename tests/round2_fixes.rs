/// Round-2 code-review regression tests.
///
/// Covers findings: #1 (XML escaping), #4 (auth defaults), #7 (namespace-aware
/// extraction + entity decode), #8 (optional services), #11 (discovery stable
/// UUID + non-probe guard), #12 (fault detail redaction).
use bytes::Bytes;
use onvif_server::service::{
    device::DeviceServiceHandler, events::EventServiceHandler, media::MediaServiceHandler,
    ptz::PTZServiceHandler,
};
use onvif_server::{
    DeviceInfo, DeviceService, EventService, MediaService, NetworkInterface, OnvifServer,
    PTZService,
};
use soap_server::SoapHandler;
use std::sync::Arc;

// ─── Shared stub impls ────────────────────────────────────────────────────────

struct StubDevice {
    info: DeviceInfo,
}

#[async_trait::async_trait]
impl DeviceService for StubDevice {
    async fn get_device_information(&self) -> Result<DeviceInfo, onvif_server::OnvifError> {
        Ok(self.info.clone())
    }

    async fn get_network_interfaces(
        &self,
    ) -> Result<Vec<NetworkInterface>, onvif_server::OnvifError> {
        Ok(vec![NetworkInterface {
            token: "eth&0".into(), // & in token — must be escaped as attr
            enabled: true,
            name: "eth<0>".into(), // < > in name — must be escaped as text
            hw_address: "DE:AD:BE:EF:\"0\":1".into(), // " in hw — must be escaped
            mtu: 1500,
        }])
    }
}

struct StubMedia;
#[async_trait::async_trait]
impl MediaService for StubMedia {
    async fn get_stream_uri(&self, _token: &str) -> Result<String, onvif_server::OnvifError> {
        Ok("rtsp://example.com/stream?foo=bar&baz=<qux>".into())
    }
    async fn get_snapshot_uri(&self, _token: &str) -> Result<String, onvif_server::OnvifError> {
        Ok("http://example.com/snap?a=1&b=2".into())
    }
}

struct StubEvent;
#[async_trait::async_trait]
impl EventService for StubEvent {}

fn make_device_handler_special() -> DeviceServiceHandler {
    DeviceServiceHandler::new(
        Arc::new(StubDevice {
            info: DeviceInfo {
                manufacturer: "Acme & Sons".into(),
                model: "Cam<1>".into(),
                firmware_version: "1\"0\"0".into(),
                serial_number: "SN-001".into(),
                hardware_id: "HW-A".into(),
            },
        }),
        "http://host/device",
        "http://host/media",
        "http://host/ptz",
        "http://host/imaging",
        "http://host/events",
    )
}

// ─── Finding #1: XML escaping ─────────────────────────────────────────────────

/// Device info with `& < > "` in trait-provided strings must produce well-formed XML.
#[tokio::test]
async fn xml_escaping_device_info_special_chars() {
    let handler = make_device_handler_special();
    let body = Bytes::from_static(
        b"<tds:GetDeviceInformation xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("handler must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    // Must be parseable as well-formed XML.
    assert_well_formed(&xml);

    // Escaped sequences must appear verbatim.
    assert!(xml.contains("Acme &amp; Sons"), "& not escaped: {xml}");
    assert!(xml.contains("Cam&lt;1&gt;"), "< > not escaped: {xml}");
    assert!(xml.contains("1&quot;0&quot;0"), "\" not escaped: {xml}");
}

/// Network interface token (attribute value) and name (element text) with special
/// characters must be escaped appropriately.
#[tokio::test]
async fn xml_escaping_network_interface_special_chars() {
    let handler = make_device_handler_special();
    let body = Bytes::from_static(
        b"<tds:GetNetworkInterfaces xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );
    let result = handler.handle(body).await.expect("handler must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert_well_formed(&xml);
    // token attribute must use escape_attr (& -> &amp;)
    assert!(
        xml.contains("token=\"eth&amp;0\""),
        "token attr not escaped: {xml}"
    );
    // name text must use escape_text
    assert!(xml.contains("eth&lt;0&gt;"), "name text not escaped: {xml}");
    // hw_address text
    assert!(
        xml.contains("&quot;0&quot;"),
        "hw_address \" not escaped: {xml}"
    );
}

/// Stream URI with & must be escaped as element text.
#[tokio::test]
async fn xml_escaping_media_stream_uri() {
    let handler = MediaServiceHandler::new(Arc::new(StubMedia), "http://host/media");
    let body = Bytes::from(
        r#"<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
           <trt:ProfileToken>profile1</trt:ProfileToken>
         </trt:GetStreamUri>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetStreamUri must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert_well_formed(&xml);
    assert!(
        xml.contains("foo=bar&amp;baz=&lt;qux&gt;"),
        "URI not escaped: {xml}"
    );
}

/// Snapshot URI with & must be escaped as element text.
#[tokio::test]
async fn xml_escaping_media_snapshot_uri() {
    let handler = MediaServiceHandler::new(Arc::new(StubMedia), "http://host/media");
    let body = Bytes::from(
        r#"<trt:GetSnapshotUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
           <trt:ProfileToken>profile1</trt:ProfileToken>
         </trt:GetSnapshotUri>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetSnapshotUri must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert_well_formed(&xml);
    assert!(
        xml.contains("a=1&amp;b=2"),
        "snapshot URI not escaped: {xml}"
    );
}

/// Preset token (attr) and name (text) with special chars must be escaped.
#[tokio::test]
async fn xml_escaping_ptz_set_preset_token() {
    struct PresetPTZ;
    #[async_trait::async_trait]
    impl PTZService for PresetPTZ {
        async fn set_preset(
            &self,
            _profile: &str,
            _name: Option<&str>,
            _token: Option<&str>,
        ) -> Result<String, onvif_server::OnvifError> {
            // Return a token that contains XML special chars.
            Ok("tok<&>en".into())
        }
    }

    let handler = PTZServiceHandler::new(Arc::new(PresetPTZ));
    let body = Bytes::from(
        r#"<tptz:SetPreset xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
             <tptz:ProfileToken>profile1</tptz:ProfileToken>
             <tptz:PresetName>MyPreset</tptz:PresetName>
           </tptz:SetPreset>"#,
    );
    let result = handler.handle(body).await.expect("SetPreset must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert_well_formed(&xml);
    assert!(
        xml.contains("tok&lt;&amp;&gt;en"),
        "preset token not escaped: {xml}"
    );
}

/// CreatePullPointSubscription: xaddr in response must be escaped.
#[tokio::test]
async fn xml_escaping_events_xaddr() {
    let handler = EventServiceHandler::new(Arc::new(StubEvent), "http://host&with&amps/events");
    let body = Bytes::from(
        r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("CreatePullPointSubscription must succeed");
    let xml = String::from_utf8(result.to_vec()).unwrap();

    assert_well_formed(&xml);
    assert!(
        xml.contains("host&amp;with&amp;amps"),
        "xaddr not escaped in subscription response: {xml}"
    );
}

// ─── Finding #4: Auth defaults ────────────────────────────────────────────────

/// Without `.auth()`, a non-bypassed operation must succeed (unauthenticated server).
#[tokio::test]
async fn auth_default_unauthenticated_server_allows_all_ops() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        // NO .auth() call — unauthenticated mode
        .device_service(StubDevice {
            info: DeviceInfo {
                manufacturer: "Test".into(),
                model: "Test".into(),
                firmware_version: "0".into(),
                serial_number: "0".into(),
                hardware_id: "0".into(),
            },
        })
        .build()
        .expect("build must succeed without auth");

    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // GetDeviceInformation is NOT in auth_bypass — in unauthenticated mode it must succeed
    // without any WS-Security header.
    let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let request = format!(
        "POST /onvif/device_service HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        soap_body.len(),
        soap_body
    );

    let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .expect("must connect");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let resp = String::from_utf8_lossy(&response);

    assert!(
        resp.starts_with("HTTP/1.1 200"),
        "unauthenticated server must allow GetDeviceInformation without credentials, got: {resp}"
    );
    assert!(
        resp.contains("GetDeviceInformationResponse"),
        "response body must contain GetDeviceInformationResponse, got: {resp}"
    );
}

/// With `.auth()`, a non-bypassed operation without credentials must be rejected.
#[tokio::test]
async fn auth_configured_server_rejects_unauthenticated_non_bypass_op() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        .auth("admin", "secret")
        .device_service(StubDevice {
            info: DeviceInfo {
                manufacturer: "Test".into(),
                model: "Test".into(),
                firmware_version: "0".into(),
                serial_number: "0".into(),
                hardware_id: "0".into(),
            },
        })
        .build()
        .expect("build must succeed");

    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let request = format!(
        "POST /onvif/device_service HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        soap_body.len(),
        soap_body
    );

    let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let resp = String::from_utf8_lossy(&response);

    // Must NOT be 200 with a success body (should be 4xx or SOAP fault).
    assert!(
        !resp.starts_with("HTTP/1.1 200") || resp.contains("Fault"),
        "auth-enabled server must reject unauthenticated GetDeviceInformation, got: {resp}"
    );
}

// ─── Finding #7: Namespace-aware extraction + entity decode ──────────────────

/// A `ProfileToken` element in the WRONG namespace must not be accepted —
/// the handler should return a fault (element not found) instead.
#[tokio::test]
async fn ns_aware_wrong_namespace_rejected() {
    let handler = MediaServiceHandler::new(Arc::new(StubMedia), "http://host/media");
    // ProfileToken is in a non-ONVIF namespace — should not match.
    let body = Bytes::from(
        r#"<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
             <wrong:ProfileToken xmlns:wrong="http://wrong.namespace.example/">profile1</wrong:ProfileToken>
           </trt:GetStreamUri>"#,
    );
    let result = handler.handle(body).await;
    assert!(
        result.is_err(),
        "ProfileToken in wrong namespace must not be accepted; expected fault"
    );
}

/// A value containing `&amp;` in the request body must reach the handler decoded as `&`.
#[tokio::test]
async fn ns_aware_entity_decoded_to_handler() {
    struct CapturingMedia {
        captured: std::sync::Mutex<String>,
    }
    #[async_trait::async_trait]
    impl MediaService for CapturingMedia {
        async fn get_stream_uri(&self, token: &str) -> Result<String, onvif_server::OnvifError> {
            *self.captured.lock().unwrap() = token.to_string();
            Ok("rtsp://example.com/stream".into())
        }
    }

    let capturing = Arc::new(CapturingMedia {
        captured: std::sync::Mutex::new(String::new()),
    });
    let handler = MediaServiceHandler::new(capturing.clone(), "http://host/media");

    // ProfileToken contains `&amp;` — handler must receive `&`.
    let body = Bytes::from(
        r#"<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
             <trt:ProfileToken>tok&amp;en</trt:ProfileToken>
           </trt:GetStreamUri>"#,
    );
    handler
        .handle(body)
        .await
        .expect("GetStreamUri must succeed");
    let received = capturing.captured.lock().unwrap().clone();
    assert_eq!(
        received, "tok&en",
        "entity &amp; must be decoded to & before reaching handler, got: {received:?}"
    );
}

// ─── Finding #8: Optional services ───────────────────────────────────────────

/// A server with only device_service must build and run() successfully — no
/// other services required.
#[tokio::test]
async fn optional_services_device_only_runs() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        .device_service(StubDevice {
            info: DeviceInfo {
                manufacturer: "T".into(),
                model: "T".into(),
                firmware_version: "0".into(),
                serial_number: "0".into(),
                hardware_id: "0".into(),
            },
        })
        .build()
        .expect("build with device_service only must succeed");

    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // GetSystemDateAndTime must respond — it's the simplest auth-bypass op.
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
        .expect("must connect");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let resp = String::from_utf8_lossy(&response);

    assert!(
        resp.starts_with("HTTP/1.1 200"),
        "device-only server must serve GetSystemDateAndTime, got: {resp}"
    );
}

/// device+media server must build and serve both routes.
#[tokio::test]
async fn optional_services_device_and_media_runs() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = OnvifServer::builder()
        .port(port)
        .device_service(StubDevice {
            info: DeviceInfo {
                manufacturer: "T".into(),
                model: "T".into(),
                firmware_version: "0".into(),
                serial_number: "0".into(),
                hardware_id: "0".into(),
            },
        })
        .media_service(StubMedia)
        // No PTZ, imaging, or events
        .build()
        .expect("build with device+media must succeed");

    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // GetProfiles on media service must succeed.
    let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <trt:GetProfiles xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let request = format!(
        "POST /onvif/media_service HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        soap_body.len(),
        soap_body
    );

    let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .expect("must connect");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let resp = String::from_utf8_lossy(&response);

    assert!(
        resp.starts_with("HTTP/1.1 200"),
        "device+media server must serve GetProfiles, got: {resp}"
    );
}

// ─── Finding #11: WS-Discovery ───────────────────────────────────────────────

// Discovery tests exercise the probe-parsing logic and stable UUID via the
// public `discovery_is_probe` / `discovery_build_probe_match` wrappers, which
// are always compiled (the underlying logic is pure XML). The UDP socket logic
// is still guarded by `#[cfg(feature = "discovery")]`.

/// A non-probe UDP payload must NOT be treated as a probe.
#[test]
fn discovery_non_probe_not_matched() {
    // Contains "Probe" as bytes but not a valid WS-Discovery Probe XML.
    let not_a_probe = b"GET /Probe HTTP/1.1\r\nHost: example.com\r\n\r\n";
    assert!(
        !onvif_server::discovery_is_probe(not_a_probe),
        "HTTP request containing 'Probe' must not be matched as WS-Discovery Probe"
    );
}

/// A SOAP Probe element in the WRONG namespace must not match.
#[test]
fn discovery_soap_wrong_ns_not_matched() {
    let body = br#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <wrong:Probe xmlns:wrong="http://wrong.example.com/"/>
  </s:Body>
</s:Envelope>"#;
    assert!(
        !onvif_server::discovery_is_probe(body),
        "Probe element in wrong namespace must not be matched"
    );
}

/// A genuine WS-Discovery Probe in the correct namespace must match.
#[test]
fn discovery_genuine_probe_matched() {
    let body = br#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery">
  <s:Body>
    <d:Probe/>
  </s:Body>
</s:Envelope>"#;
    assert!(
        onvif_server::discovery_is_probe(body),
        "genuine WS-Discovery Probe must be matched"
    );
}

/// Two probe responses with the same device UUID must have the same
/// EndpointReference address — stable identity.
#[test]
fn discovery_stable_uuid_across_probes() {
    let device_uuid = uuid::Uuid::new_v4();
    let r1 = onvif_server::discovery_build_probe_match(
        "urn:uuid:msg-1",
        "http://host/device",
        device_uuid,
    );
    let r2 = onvif_server::discovery_build_probe_match(
        "urn:uuid:msg-2",
        "http://host/device",
        device_uuid,
    );

    let extract_addr = |xml: &str| -> Option<String> {
        let start = xml.find("<a:Address>")? + "<a:Address>".len();
        let end = xml[start..].find("</a:Address>")? + start;
        Some(xml[start..end].to_string())
    };

    let addr1 = extract_addr(&r1).expect("EndpointReference address missing in probe 1");
    let addr2 = extract_addr(&r2).expect("EndpointReference address missing in probe 2");
    assert_eq!(
        addr1, addr2,
        "EndpointReference must be stable across probe responses"
    );
    assert!(
        addr1.contains(&device_uuid.to_string()),
        "EndpointReference must contain the device UUID: {addr1}"
    );
}

// ─── Finding #12: Fault detail redaction ─────────────────────────────────────

/// An unknown-subscription fault reason must NOT echo the subscription ID back.
#[tokio::test]
async fn fault_unknown_subscription_does_not_echo_id() {
    let handler = EventServiceHandler::new(Arc::new(StubEvent), "http://host/events");

    let secret_id = "very-secret-subscription-id-12345";
    let body = Bytes::from(format!(
        r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
             <tev:SubscriptionId>{secret_id}</tev:SubscriptionId>
           </tev:PullMessages>"#
    ));
    let result = handler.handle(body).await;
    assert!(result.is_err(), "unknown subscription must return a fault");
    let fault = result.unwrap_err();
    let reason = fault.reason;
    assert!(
        !reason.contains(secret_id),
        "fault reason must NOT echo the subscription id, got reason: {reason}"
    );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Parse XML and assert it is well-formed (no parse error).
fn assert_well_formed(xml: &str) {
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => panic!("XML not well-formed: {e}\n\nXML was:\n{xml}"),
            _ => {}
        }
    }
}
