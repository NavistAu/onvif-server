//! Deterministic controlled ONVIF camera fixture (spec §7).
//!
//! `ControlledCamera` implements all five service traits with pinned values so
//! Layer-1 snapshots are stable between runs.  Volatile fields (timestamps,
//! UUIDs) are emitted by the handler-level code and masked by the §8 mask
//! table — they cannot be pinned through the trait API.
//!
//! ## Handler-internal static values (NOT controllable via trait)
//!
//! Several PTZ discovery operations (GetNodes, GetConfigurations, GetNode,
//! GetConfiguration, GetConfigurationOptions, GetServiceCapabilities) and
//! Media operations (GetVideoSources, GetVideoSourceConfigurations,
//! GetVideoEncoderConfigurations) emit static XML that is assembled by the
//! handler using crate-level constants, bypassing the trait entirely.  Those
//! constants pin the following tokens:
//!
//! | Constant               | Value          |
//! |------------------------|----------------|
//! | `PROFILE_TOKEN`        | `profile_0`    |
//! | `VIDEO_SOURCE_TOKEN`   | `video_src_0`  |
//! | `VIDEO_ENCODER_TOKEN`  | `video_enc_0`  |
//! | `PTZ_NODE_TOKEN`       | `ptz_node_0`   |
//! | `PTZ_CONFIG_TOKEN`     | `ptz_cfg_0`    |
//!
//! These are already deterministic (they are compile-time constants), so they
//! satisfy the §7 determinism requirement even though the fixture cannot
//! override them.  The spec §7 table lists the intended names; the actual
//! values emitted are those above.

use async_trait::async_trait;
use onvif_server::generated::{
    DeviceInfo, HostnameInformation, ImagingSettings, NetworkInterface, PTZPreset, PTZStatusResult,
    Scope, ScopeDefinition,
};
use onvif_server::{DeviceService, EventService, ImagingService, MediaService, OnvifError};
use onvif_server::{MediaProfile, PTZService};

// ---------------------------------------------------------------------------
// Pinned §7 constants (what the fixture controls directly through the trait)
// ---------------------------------------------------------------------------

const MANUFACTURER: &str = "Crossref";
const MODEL: &str = "Controlled-1";
const FIRMWARE: &str = "1.0.0";
const SERIAL: &str = "CR-0001";
const HARDWARE_ID: &str = "CR-HW-1";

const HOSTNAME: &str = "controlled-onvif";

/// Profile token (matches `onvif_server::constants::PROFILE_TOKEN`).
const PROFILE_TOKEN: &str = "profile_0";
/// Video source token (matches `onvif_server::constants::VIDEO_SOURCE_TOKEN`).
const VIDEO_SOURCE_TOKEN: &str = "video_src_0";

const PRESET_TOKEN: &str = "preset_1";
const PRESET_NAME: &str = "Home";

// ---------------------------------------------------------------------------
// Shared fixture constants used by both the Layer-1 and Layer-2 harnesses
// ---------------------------------------------------------------------------

/// Stable WS-Discovery EndpointReference UUID for the controlled fixture (F-7).
///
/// The controlled server binary pins `discovery_uuid` to this value via
/// `OnvifServerBuilder::discovery_uuid(CONTROLLED_DISCOVERY_UUID)` so every
/// discovery cycle advertises the same endpoint identity.  The Layer-1 harness
/// sources its `expected_endpoint` from this constant so the assertion is
/// deterministic and non-vacuous.
///
/// Value: `b5e2d6f0-0000-0000-0000-000000000001`
pub const CONTROLLED_DISCOVERY_UUID: uuid::Uuid = uuid::Uuid::from_bytes([
    0xb5, 0xe2, 0xd6, 0xf0, // time_low
    0x00, 0x00, // time_mid
    0x00, 0x00, // time_hi_and_version
    0x00, 0x00, // clock_seq
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // node
]);

/// The four §7 fixture scopes emitted by `ControlledCamera::get_scopes` and
/// validated by the `scopes_match_fixture` invariant.  Both `InvariantCtx`
/// construction sites (`layer1_replay::default_ctx` and
/// `layer2::validate_response`) must reference this slice so the expected list
/// never diverges from the fixture definition.
pub const FIXTURE_SCOPES: &[&str] = &[
    "onvif://www.onvif.org/Profile/Streaming",
    "onvif://www.onvif.org/type/video_encoder",
    "onvif://www.onvif.org/name/Controlled",
    "onvif://www.onvif.org/location/lab",
];

// ---------------------------------------------------------------------------
// ControlledCamera
// ---------------------------------------------------------------------------

/// A fully deterministic ONVIF camera fixture implementing all five service
/// traits with the §7 pinned values.
///
/// `host` is the advertised host string (e.g. `"controlled-onvif:8080"`) used
/// to construct stream/snapshot URIs that match what the server advertises.
#[derive(Clone)]
pub struct ControlledCamera {
    host: String,
}

impl ControlledCamera {
    /// Create a new fixture with the default advertised host
    /// `"controlled-onvif:8080"` (used by the controlled server binary).
    pub fn new() -> Self {
        Self {
            host: "controlled-onvif:8080".to_string(),
        }
    }

    /// Create a fixture with a custom advertised host (for in-process tests
    /// where the TestServer binds to an ephemeral address, but the stream/
    /// snapshot URIs still need a deterministic value for Layer-1 snapshots).
    pub fn with_host(host: impl Into<String>) -> Self {
        Self { host: host.into() }
    }

    fn stream_uri(&self) -> String {
        format!("rtsp://{}/stream0", self.host)
    }

    fn snapshot_uri(&self) -> String {
        format!("http://{}/snapshot0", self.host)
    }
}

impl Default for ControlledCamera {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DeviceService
// ---------------------------------------------------------------------------

#[async_trait]
impl DeviceService for ControlledCamera {
    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        Ok(DeviceInfo {
            manufacturer: MANUFACTURER.to_string(),
            model: MODEL.to_string(),
            firmware_version: FIRMWARE.to_string(),
            serial_number: SERIAL.to_string(),
            hardware_id: HARDWARE_ID.to_string(),
        })
    }

    async fn get_scopes(&self) -> Result<Vec<Scope>, OnvifError> {
        Ok(vec![
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/Profile/Streaming".to_string(),
            },
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/type/video_encoder".to_string(),
            },
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/name/Controlled".to_string(),
            },
            Scope {
                scope_def: ScopeDefinition::Fixed,
                scope_item: "onvif://www.onvif.org/location/lab".to_string(),
            },
        ])
    }

    async fn get_hostname(&self) -> Result<HostnameInformation, OnvifError> {
        Ok(HostnameInformation {
            from_dhcp: false,
            name: Some(HOSTNAME.to_string()),
        })
    }

    async fn get_network_interfaces(&self) -> Result<Vec<NetworkInterface>, OnvifError> {
        Ok(vec![NetworkInterface {
            token: "eth0".to_string(),
            enabled: true,
            name: "eth0".to_string(),
            hw_address: "00:11:22:33:44:55".to_string(),
            mtu: 1500,
        }])
    }
    // get_system_date_and_time: use default (chrono::Utc::now()) — volatile, masked by §8.
}

// ---------------------------------------------------------------------------
// MediaService
// ---------------------------------------------------------------------------
//
// `profiles()` is a synchronous fn on the trait; `get_stream_uri` and
// `get_snapshot_uri` are async.  Since the trait is annotated with
// `#[async_trait]` all three are overridden in a single impl block.

#[async_trait]
impl MediaService for ControlledCamera {
    fn profiles(&self) -> Vec<MediaProfile> {
        vec![MediaProfile {
            token: PROFILE_TOKEN.to_string(),
            name: "ControlledProfile".to_string(),
            video_source_token: VIDEO_SOURCE_TOKEN.to_string(),
            width: 1920,
            height: 1080,
            encoding: "H264".to_string(),
            framerate: 25,
            bitrate: 2048,
        }]
    }

    async fn get_stream_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok(self.stream_uri())
    }

    async fn get_snapshot_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok(self.snapshot_uri())
    }
}

// ---------------------------------------------------------------------------
// ImagingService
// ---------------------------------------------------------------------------

#[async_trait]
impl ImagingService for ControlledCamera {
    /// Returns settings that cause the handler to emit exactly ONE
    /// `<tt:WhiteBalance>` element (BLOCK-OS-C06 regression lock).
    async fn get_imaging_settings(
        &self,
        _video_source_token: String,
    ) -> Result<ImagingSettings, OnvifError> {
        Ok(ImagingSettings {
            brightness: Some(50.0),
            contrast: Some(50.0),
            sharpness: Some(50.0),
            color_saturation: None,
            // Both gains present → handler wraps them in a single <tt:WhiteBalance>.
            white_balance_cr_gain: Some(1.0),
            white_balance_cb_gain: Some(1.0),
        })
    }
}

// ---------------------------------------------------------------------------
// PTZService
// ---------------------------------------------------------------------------
//
// Discovery operations (GetNodes, GetNode, GetConfigurations, GetConfiguration,
// GetConfigurationOptions, GetServiceCapabilities) are handler-internal static
// XML assembled from crate constants — they are NOT on the trait and cannot be
// overridden here.

#[async_trait]
impl PTZService for ControlledCamera {
    async fn get_status(&self, _profile_token: &str) -> Result<PTZStatusResult, OnvifError> {
        Ok(PTZStatusResult {
            pan_tilt_moving: false,
            zoom_moving: false,
        })
    }

    async fn get_presets(&self, _profile_token: &str) -> Result<Vec<PTZPreset>, OnvifError> {
        Ok(vec![PTZPreset {
            token: PRESET_TOKEN.to_string(),
            name: PRESET_NAME.to_string(),
        }])
    }

    async fn goto_preset(
        &self,
        _profile_token: &str,
        _preset_token: &str,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn relative_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn absolute_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn continuous_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    async fn stop(
        &self,
        _profile_token: &str,
        _pan_tilt: bool,
        _zoom: bool,
    ) -> Result<(), OnvifError> {
        Ok(())
    }

    // set_preset and remove_preset use the default not_implemented() — the
    // fixture is read-oriented; mutating ops are not needed for Layer-1.
}

// ---------------------------------------------------------------------------
// EventService
// ---------------------------------------------------------------------------
//
// GetEventProperties, CreatePullPointSubscription, PullMessages, and
// Unsubscribe are all handler-internal.  The EventService trait exposes only
// get_event_properties(), which the handler ignores in favour of static XML.
// We accept the default not_implemented() — the handler never calls this method.

#[async_trait]
impl EventService for ControlledCamera {
    async fn get_event_properties(&self) -> Result<(), OnvifError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use onvif_server::OnvifServer;

    /// Build a controlled SUT using `into_router()` + `axum_test::TestServer`.
    ///
    /// The fixture is configured with a fixed advertised host so that
    /// stream/snapshot URIs are deterministic in Layer-1 snapshots.
    fn build_test_server() -> TestServer {
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
            .expect("build");
        let router = server.into_router().expect("into_router");
        TestServer::new(router)
    }

    /// Build a WS-Security UsernameToken header for admin/admin with a fixed
    /// nonce and created timestamp (deterministic for tests).
    fn ws_security_header(nonce_b64: &str, created: &str) -> String {
        let digest =
            soap_server::compute_digest(nonce_b64, created, "admin").expect("compute_digest");
        format!(
            r#"<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
  <wsse:UsernameToken>
    <wsse:Username>admin</wsse:Username>
    <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{digest}</wsse:Password>
    <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{nonce_b64}</wsse:Nonce>
    <wsu:Created>{created}</wsu:Created>
  </wsse:UsernameToken>
</wsse:Security>"#,
            digest = digest,
            nonce_b64 = nonce_b64,
            created = created,
        )
    }

    /// Wrap a SOAP body + optional Security header into a SOAP 1.2 envelope.
    fn soap_envelope(security_header: Option<&str>, body_content: &str) -> String {
        let header_block = if let Some(sec) = security_header {
            format!("<s:Header>{}</s:Header>", sec)
        } else {
            String::new()
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  {header}
  <s:Body>{body}</s:Body>
</s:Envelope>"#,
            header = header_block,
            body = body_content,
        )
    }

    // ── Smoke test 1: GetSystemDateAndTime (auth-bypassed) ────────────────────

    #[tokio::test]
    async fn get_system_date_and_time_unauthenticated_succeeds() {
        let ts = build_test_server();
        let body = soap_envelope(
            None,
            r#"<tds:GetSystemDateAndTime xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>"#,
        );
        let resp = ts
            .post("/onvif/device_service")
            .content_type("application/soap+xml; charset=utf-8")
            .bytes(bytes::Bytes::from(body))
            .await;
        assert_eq!(resp.status_code().as_u16(), 200);
        let text = resp.text();
        assert!(
            text.contains("GetSystemDateAndTimeResponse"),
            "expected GetSystemDateAndTimeResponse, got: {text}"
        );
    }

    // ── Smoke test 2: GetDeviceInformation (authed) with pinned assertions ────

    #[tokio::test]
    async fn get_device_information_returns_pinned_values() {
        // Use a fresh nonce+timestamp for each test (unique nonce rule).
        // The WS-Security freshness window is 300s — use Utc::now() so the token is valid.
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        let nonce_bytes = b"fixture-test-nonce-1";
        let nonce_b64 = BASE64.encode(nonce_bytes);
        let created = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let security = ws_security_header(&nonce_b64, &created);

        let ts = build_test_server();
        let body = soap_envelope(
            Some(&security),
            r#"<tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>"#,
        );
        let resp = ts
            .post("/onvif/device_service")
            .content_type("application/soap+xml; charset=utf-8")
            .bytes(bytes::Bytes::from(body))
            .await;
        assert_eq!(resp.status_code().as_u16(), 200);
        let text = resp.text();
        assert!(
            text.contains("Crossref"),
            "expected Manufacturer=Crossref, got: {text}"
        );
        assert!(
            text.contains("Controlled-1"),
            "expected Model=Controlled-1, got: {text}"
        );
        assert!(
            text.contains("CR-0001"),
            "expected SerialNumber=CR-0001, got: {text}"
        );
    }

    // ── Smoke test 3: GetImagingSettings → exactly ONE <tt:WhiteBalance> ─────

    #[tokio::test]
    async fn get_imaging_settings_has_exactly_one_white_balance() {
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        let nonce_bytes = b"fixture-test-nonce-2";
        let nonce_b64 = BASE64.encode(nonce_bytes);
        let created = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let security = ws_security_header(&nonce_b64, &created);

        let ts = build_test_server();
        let body = soap_envelope(
            Some(&security),
            r#"<timg:GetImagingSettings xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl">
  <timg:VideoSourceToken>video_src_0</timg:VideoSourceToken>
</timg:GetImagingSettings>"#,
        );
        let resp = ts
            .post("/onvif/imaging_service")
            .content_type("application/soap+xml; charset=utf-8")
            .bytes(bytes::Bytes::from(body))
            .await;
        assert_eq!(resp.status_code().as_u16(), 200);
        let text = resp.text();
        // Count occurrences of <tt:WhiteBalance> (opening tag only, not </tt:WhiteBalance>)
        let count = text.matches("<tt:WhiteBalance>").count();
        assert_eq!(
            count, 1,
            "expected exactly 1 <tt:WhiteBalance> element, found {count} in: {text}"
        );
    }

    // ── Smoke test 4: PTZ GetServiceCapabilities → MoveStatus="true" ─────────
    //
    // GetServiceCapabilities is NOT declared in the PTZ WSDL, so the SOAP router
    // (which dispatches based on WSDL operations) blocks it before it reaches the
    // handler.  We test the handler directly here, matching the approach of the
    // onvif-server ptz_service.rs unit tests.  This is sufficient to regression-lock
    // the MoveStatus-as-attribute fix (BLOCK-OS-C06); the Layer-1 scenario set
    // exercises it at the handler level via the invariant registry.

    #[tokio::test]
    async fn ptz_get_service_capabilities_has_move_status_attr() {
        use bytes::Bytes;
        use onvif_server::PTZServiceHandler;
        use soap_server::SoapHandler;

        let cam = ControlledCamera::with_host("controlled-onvif:8080");
        let handler = PTZServiceHandler::new(std::sync::Arc::new(cam));

        let body = Bytes::from(
            r#"<tptz:GetServiceCapabilities xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>"#,
        );
        let result = handler
            .handle(body)
            .await
            .expect("GetServiceCapabilities must not return SoapFault");
        let text = String::from_utf8(result.to_vec()).expect("utf8");
        assert!(
            text.contains(r#"MoveStatus="true""#),
            "expected MoveStatus=\"true\" attribute, got: {text}"
        );
    }
}
