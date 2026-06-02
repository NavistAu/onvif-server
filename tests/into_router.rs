// tests/into_router.rs
// Acceptance tests for OnvifServer::into_router() — Task 2 of crossref Phase 2a.
//
// These tests drive the FULL SOAP/auth/routing stack in-process via axum_test::TestServer
// without binding a real port.

use axum_test::TestServer;
use onvif_server::{
    DeviceInfo, DeviceService, EventService, ImagingService, MediaService, OnvifServer, PTZService,
};

// ---------------------------------------------------------------------------
// Minimal stub service impls (reused pattern from tests/device_management.rs)
// ---------------------------------------------------------------------------

struct StubDevice;

#[async_trait::async_trait]
impl DeviceService for StubDevice {
    async fn get_device_information(&self) -> Result<DeviceInfo, onvif_server::OnvifError> {
        Ok(DeviceInfo {
            manufacturer: "StubMfr".into(),
            model: "StubModel".into(),
            firmware_version: "0.0.1".into(),
            serial_number: "STUB-001".into(),
            hardware_id: "STUB-HW".into(),
        })
    }
}

struct StubMedia;
#[async_trait::async_trait]
impl MediaService for StubMedia {}

struct StubPTZ;
#[async_trait::async_trait]
impl PTZService for StubPTZ {}

struct StubImaging;
#[async_trait::async_trait]
impl ImagingService for StubImaging {}

struct StubEvent;
#[async_trait::async_trait]
impl EventService for StubEvent {}

// ---------------------------------------------------------------------------
// Minimal SOAP envelopes for each service — enough for dispatch to accept and
// route (auth-exempt op to avoid WS-Security complexity in the mount check).
// ---------------------------------------------------------------------------

/// GetSystemDateAndTime is auth-bypassed by default — safe to use without credentials.
const DEVICE_SOAP: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetSystemDateAndTime xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// GetVideoSources — routable media op; no auth required (server built without auth).
const MEDIA_SOAP: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <trt:GetVideoSources xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// GetNodes — routable PTZ op; no auth required.
const PTZ_SOAP: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tptz:GetNodes xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// GetImagingSettings with a dummy token — routable imaging op; no auth required.
const IMAGING_SOAP: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <timg:GetImagingSettings xmlns:timg="http://www.onvif.org/ver10/imaging/wsdl">
      <timg:VideoSourceToken>vsrc_0</timg:VideoSourceToken>
    </timg:GetImagingSettings>
  </s:Body>
</s:Envelope>"#;

/// GetEventProperties — routable events op; no auth required.
const EVENTS_SOAP: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tev:GetEventProperties xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>
  </s:Body>
</s:Envelope>"#;

// ---------------------------------------------------------------------------
// Test 1: all 5 services mounted — none returns 404
// ---------------------------------------------------------------------------

#[tokio::test]
async fn into_router_serves_all_registered_services() {
    let server = OnvifServer::builder()
        .port(0)
        .device_service(StubDevice)
        .media_service(StubMedia)
        .ptz_service(StubPTZ)
        .imaging_service(StubImaging)
        .event_service(StubEvent)
        .build()
        .expect("build must succeed");

    let router = server.into_router().expect("into_router must succeed");
    let ts = TestServer::new(router);

    let paths_and_bodies: &[(&str, &[u8])] = &[
        ("/onvif/device_service", DEVICE_SOAP),
        ("/onvif/media_service", MEDIA_SOAP),
        ("/onvif/ptz_service", PTZ_SOAP),
        ("/onvif/imaging_service", IMAGING_SOAP),
        ("/onvif/events_service", EVENTS_SOAP),
    ];

    for (path, body) in paths_and_bodies {
        let resp = ts
            .post(path)
            .content_type("application/soap+xml; charset=utf-8")
            .bytes(bytes::Bytes::copy_from_slice(body))
            .await;
        let status = resp.status_code().as_u16();
        assert_ne!(
            status, 404,
            "service not mounted at {path}: got status {status}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2: full-stack auth + routing — WS-Security middleware and fault routing
//         run through the stack (not just handler-level logic).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn into_router_full_stack_auth_and_routing() {
    // Build with auth so that most ops require WS-Security credentials.
    let server = OnvifServer::builder()
        .port(0)
        .auth("admin", "admin")
        .device_service(StubDevice)
        .media_service(StubMedia)
        .ptz_service(StubPTZ)
        .imaging_service(StubImaging)
        .event_service(StubEvent)
        .build()
        .expect("build must succeed");

    let router = server.into_router().expect("into_router must succeed");
    let ts = TestServer::new(router);

    // --- 2a: Protected op with NO WS-Security header → SOAP fault (not 200, not 404) ---
    // GetDeviceInformation requires auth; omitting WS-Security must yield a SOAP fault.
    let protected_soap = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let resp = ts
        .post("/onvif/device_service")
        .content_type("application/soap+xml; charset=utf-8")
        .bytes(bytes::Bytes::copy_from_slice(protected_soap))
        .await;

    let status = resp.status_code().as_u16();
    let body = resp.text();

    // Must not be 404 (route is mounted) and must not be a plain 200 success —
    // the auth middleware must have intervened and produced a SOAP fault.
    assert_ne!(
        status, 404,
        "device_service must be mounted (not 404) for auth test"
    );
    assert_ne!(
        status, 200,
        "protected op without credentials must not return 200 success"
    );

    // The response must contain a SOAP fault element.
    assert!(
        body.contains("Fault") || body.contains("fault"),
        "unauthenticated protected op must yield a SOAP Fault, got status={status} body={body}"
    );

    // --- 2b: Auth-exempt op (GetSystemDateAndTime) must succeed even without credentials ---
    // This confirms the auth bypass list runs through the full stack correctly.
    let exempt_soap = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetSystemDateAndTime xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let resp = ts
        .post("/onvif/device_service")
        .content_type("application/soap+xml; charset=utf-8")
        .bytes(bytes::Bytes::copy_from_slice(exempt_soap))
        .await;

    let status = resp.status_code().as_u16();
    let body = resp.text();

    assert_eq!(
        status, 200,
        "GetSystemDateAndTime (auth-exempt) must return 200 without credentials, body={body}"
    );
    assert!(
        body.contains("GetSystemDateAndTimeResponse"),
        "GetSystemDateAndTime must return a valid response, got body={body}"
    );

    // --- 2c: Unknown/unroutable op → a SOAP fault (routing fault, not 404) ---
    // The server mounts specific routes; an operation the dispatch table can't route
    // must produce a SOAP fault rather than a raw 404.
    let unknown_soap = br#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:NonExistentOperation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

    let resp = ts
        .post("/onvif/device_service")
        .content_type("application/soap+xml; charset=utf-8")
        .bytes(bytes::Bytes::copy_from_slice(unknown_soap))
        .await;

    let status = resp.status_code().as_u16();
    let body = resp.text();

    assert_ne!(
        status, 404,
        "device_service must be mounted (not 404) for unknown-op test"
    );
    assert!(
        body.contains("Fault") || body.contains("fault"),
        "unknown op must yield a SOAP Fault, got status={status} body={body}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: into_router() succeeds for a device-only (minimal) server
// ---------------------------------------------------------------------------

#[tokio::test]
async fn into_router_device_only_succeeds() {
    let server = OnvifServer::builder()
        .port(0)
        .device_service(StubDevice)
        .build()
        .expect("build must succeed");

    let router = server
        .into_router()
        .expect("into_router must succeed with only device service");
    let ts = TestServer::new(router);

    // device_service must respond
    let resp = ts
        .post("/onvif/device_service")
        .content_type("application/soap+xml; charset=utf-8")
        .bytes(bytes::Bytes::copy_from_slice(DEVICE_SOAP))
        .await;
    assert_ne!(
        resp.status_code().as_u16(),
        404,
        "device_service must be mounted"
    );

    // Optional services must NOT be mounted (they weren't registered)
    for path in [
        "/onvif/media_service",
        "/onvif/ptz_service",
        "/onvif/imaging_service",
        "/onvif/events_service",
    ] {
        let resp = ts
            .post(path)
            .content_type("application/soap+xml; charset=utf-8")
            .bytes(bytes::Bytes::copy_from_slice(DEVICE_SOAP))
            .await;
        assert_eq!(
            resp.status_code().as_u16(),
            404,
            "unregistered service {path} must return 404"
        );
    }
}
