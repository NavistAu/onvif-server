//! Builds the controlled ONVIF SUT in-process (OnvifServer::into_router + axum_test) and
//! replays requests through the full SOAP/auth/routing stack.

use axum_test::TestServer;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use onvif_server::OnvifServer;

use crate::fixture::ControlledCamera;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

pub struct Sut {
    pub(crate) server: TestServer,
}

pub struct Resp {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

// ---------------------------------------------------------------------------
// SUT construction
// ---------------------------------------------------------------------------

/// Build the controlled SUT: a `ControlledCamera` wired to all five services,
/// auth admin/admin, advertised host pinned to `"controlled-onvif:8080"` for
/// deterministic Layer-1 URIs/scopes.
pub fn build_controlled_sut() -> Sut {
    let cam = ControlledCamera::with_host("controlled-onvif:8080");
    let server = OnvifServer::builder()
        .port(0)
        .advertised_host("controlled-onvif:8080")
        .auth("admin", "admin")
        .device_service(cam.clone())
        .media_service(cam.clone())
        .ptz_service(cam.clone())
        .imaging_service(cam.clone())
        .event_service(cam)
        .build()
        .expect("build_controlled_sut: OnvifServer::build failed");
    let router = server
        .into_router()
        .expect("build_controlled_sut: into_router failed");
    let ts = TestServer::new(router);
    Sut { server: ts }
}

impl Sut {
    /// POST `body` bytes to `path` with content-type `ct`.
    pub async fn post(&self, path: &str, body: &[u8], ct: &str) -> Resp {
        let r = self
            .server
            .post(path)
            .content_type(ct)
            .bytes(bytes::Bytes::copy_from_slice(body))
            .await;
        Resp {
            status: r.status_code().as_u16(),
            body: r.as_bytes().to_vec(),
        }
    }
}

/// Map a service name to its axum mount path.
pub fn service_path(service: &str) -> &'static str {
    match service {
        "device" => "/onvif/device_service",
        "media" => "/onvif/media_service",
        "imaging" => "/onvif/imaging_service",
        "ptz" => "/onvif/ptz_service",
        "events" => "/onvif/events_service",
        _ => "/onvif/device_service",
    }
}

// ---------------------------------------------------------------------------
// Dynamic WS-Security injection (Part B)
// ---------------------------------------------------------------------------

/// Build a fresh WS-Security UsernameToken header for admin/admin.
///
/// `nonce_seed` is varied per request so no two calls in the same run share
/// a nonce.  `Created` is computed from `Utc::now()` at call-time so the
/// token is always within the server's ~300 s freshness window.
pub fn build_security_header(nonce_seed: u64) -> String {
    // 16-byte nonce: 8 fixed prefix bytes + nonce_seed as little-endian u64.
    let mut nonce_bytes = [0u8; 16];
    nonce_bytes[..8].copy_from_slice(b"onvif-cr");
    nonce_bytes[8..].copy_from_slice(&nonce_seed.to_le_bytes());
    let nonce_b64 = BASE64.encode(nonce_bytes);

    let created = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    let digest =
        soap_server::compute_digest(&nonce_b64, &created, "admin").expect("compute_digest");

    format!(
        r#"<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
  <wsse:UsernameToken>
    <wsse:Username>admin</wsse:Username>
    <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{digest}</wsse:Password>
    <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{nonce_b64}</wsse:Nonce>
    <wsu:Created>{created}</wsu:Created>
  </wsse:UsernameToken>
</wsse:Security>"#
    )
}

/// Inject a freshly-built WS-Security header into an envelope that contains
/// the `<!--WSSE-->` marker inside `<env:Header><!--WSSE--></env:Header>`.
///
/// Returns the envelope bytes unchanged if the marker is absent.
pub fn inject_wsse(envelope: &[u8], nonce_seed: u64) -> Vec<u8> {
    const MARKER: &[u8] = b"<!--WSSE-->";
    if let Some(pos) = envelope.windows(MARKER.len()).position(|w| w == MARKER) {
        let header = build_security_header(nonce_seed);
        let mut out = Vec::with_capacity(envelope.len() + header.len());
        out.extend_from_slice(&envelope[..pos]);
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(&envelope[pos + MARKER.len()..]);
        out
    } else {
        envelope.to_vec()
    }
}
