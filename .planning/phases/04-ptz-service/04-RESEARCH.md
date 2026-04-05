# Phase 4: PTZ Service - Research

**Researched:** 2026-04-05
**Domain:** ONVIF Profile S PTZ Service — full control surface plus Frigate autotracker compatibility
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Porting operation following best prior art (onvif-rs, python-onvif-zeep, ONVIF spec, Frigate source) — same as Phases 1-3
- Follow established patterns from the research — PTZServiceHandler mirrors DeviceServiceHandler/MediaServiceHandler dispatch pattern
- Hand-written types (not codegen) — same approach throughout
- ServiceHandler dispatch pattern (extract_local_name + match) proven in Phases 2-3
- Token constants: PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV already defined
- Builder already accepts `.ptz_service(impl)` — needs wiring in run() like Device/Media
- Router::merge() pattern for multi-service wiring proven in Phase 3

### Claude's Discretion
- PTZServiceHandler implementation following established handler pattern
- PTZService trait signatures: typed parameters for movement commands (pan/tilt/zoom floats), typed returns for status/presets
- Type expansion for PTZ-specific structures (PTZNode, PTZConfiguration, PTZConfigurationOptions, PTZStatus, Preset, etc.)
- TranslationSpaceFov advertisement in GetNodes and GetConfigurationOptions — must use exact URI from TRANSLATION_SPACE_FOV constant per research pitfall #3
- GetServiceCapabilities MoveStatus="true" attribute — per research pitfall #4
- GetStatus response with PanTilt/Zoom IDLE/MOVING — per research
- Frigate compat test (tests/frigate_compat.rs) replaying the exact autotracker call sequence
- virtual_ptz example (examples/virtual_ptz.rs) with in-memory stub implementation
- All technical decisions follow research findings, DESIGN.md, and Frigate source as authoritative references

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PTZ-01 | GetNodes returns PTZ node(s) advertising TranslationSpaceFov in RelativePanTiltTranslationSpace | PTZNode XSD structure confirmed from onvif.xsd lines 4464-4522; PTZSpaces/RelativePanTiltTranslationSpace confirmed from lines 4902-4910; Space2DDescription (URI + XRange + YRange) from lines 4966-4990 |
| PTZ-02 | GetNode with a node token returns specific PTZ node details | Same structure as PTZ-01; requires NodeToken extraction from request body |
| PTZ-03 | GetConfigurations returns all PTZ configurations with node token references | PTZConfiguration XSD type from onvif.xsd lines 4567-4675; NodeToken element confirmed required |
| PTZ-04 | GetConfiguration with a config token returns configuration details | Same PTZConfiguration structure; requires ConfigurationToken extraction |
| PTZ-05 | GetConfigurationOptions returns supported PTZ spaces including TranslationSpaceFov with X/Y ranges | PTZConfigurationOptions XSD from lines 4758-4792; Spaces element (type PTZSpaces) with RelativePanTiltTranslationSpace confirmed; Frigate checks space["URI"] contains "TranslationSpaceFov" |
| PTZ-06 | GetServiceCapabilities returns MoveStatus="true" capability advertisement | Must return `<tptz:Capabilities MoveStatus="true"/>` — Frigate calls find_by_key(vars(capabilities), "MoveStatus") |
| PTZ-07 | RelativeMove invokes consumer trait with typed pan/tilt/zoom parameters | RelativeMove WSDL: ProfileToken + Translation (PTZVector) + optional Speed; Frigate sets Translation.PanTilt.x and .y |
| PTZ-08 | ContinuousMove invokes consumer trait with velocity parameters | ContinuousMove WSDL: ProfileToken + optional Velocity (PTZSpeed) + optional Timeout |
| PTZ-09 | Stop ceases PTZ movement with PanTilt and Zoom booleans respected | StopRequest WSDL: ProfileToken + optional PanTilt (boolean) + optional Zoom (boolean) |
| PTZ-10 | GetStatus returns MoveStatus with PanTilt and Zoom IDLE/MOVING fields | PTZStatus in common.xsd lines 98-161; PTZMoveStatus has PanTilt + Zoom both of type MoveStatus (IDLE/MOVING/UNKNOWN); Frigate reads status.MoveStatus.PanTilt and status.MoveStatus.Zoom |
| PTZ-11 | GetPresets returns consumer's preset list | GetPresets WSDL: ProfileToken in request; PTZPreset type (token attr + optional Name + optional PTZPosition) in response |
| PTZ-12 | GotoPreset invokes consumer trait with preset token | GotoPreset WSDL: ProfileToken + optional PresetToken + optional Speed |
| PTZ-13 | AbsoluteMove invokes consumer trait with position parameters | AbsoluteMove WSDL: ProfileToken + optional Destination (PTZVector) + optional Speed (PTZSpeed) |
| PTZ-14 | SetPreset invokes consumer trait to create/update preset | SetPreset WSDL: ProfileToken + optional PresetName + optional PresetToken in request; returns PresetToken |
| PTZ-15 | RemovePreset invokes consumer trait to delete preset | RemovePreset WSDL: ProfileToken + optional PresetName + optional PresetToken |
| TEST-01 | Integration test replaying Frigate autotracker call sequence | Sequence: GetProfiles (Media) → GetConfigurationOptions → GetServiceCapabilities → GetStatus → RelativeMove → GotoPreset; all in tests/frigate_compat.rs |
| TEST-02 | virtual_ptz example with all required trait methods | examples/virtual_ptz.rs implementing PTZService + DeviceService + MediaService with in-memory preset storage |
</phase_requirements>

## Summary

Phase 4 implements the ONVIF PTZ Service — 13 operations split between discovery (GetNodes, GetNode, GetConfigurations, GetConfiguration, GetConfigurationOptions, GetServiceCapabilities) and control (RelativeMove, ContinuousMove, AbsoluteMove, Stop, GetStatus, GetPresets, GotoPreset, SetPreset, RemovePreset). The implementation follows the PTZServiceHandler pattern exactly mirroring MediaServiceHandler and DeviceServiceHandler from prior phases: one struct implementing `SoapHandler`, `extract_local_name` dispatch, format-string XML responses.

The Frigate autotracker imposes four precision requirements that have no margin for approximation: (1) `GetNodes` must include `RelativePanTiltTranslationSpace` with a URI containing "TranslationSpaceFov", (2) `GetConfigurationOptions` must include the same URI in `Spaces.RelativePanTiltTranslationSpace`, (3) `GetServiceCapabilities` must return `MoveStatus="true"` as an attribute, and (4) `GetStatus` must return `MoveStatus` with `PanTilt` and `Zoom` sub-elements set to "IDLE" or "MOVING". All four are verified directly from the Frigate source (`frigate/ptz/onvif.py`).

The phase also delivers `tests/frigate_compat.rs` — the Frigate call sequence integration test — and `examples/virtual_ptz.rs` demonstrating the complete consumer API. The PTZ service mounts at `/onvif/ptz_service` and is merged into the axum router as a third `Router::merge()` call in `run()`. The `PTZService` trait stubs in `src/traits/ptz.rs` must be expanded from the current `()` return types to typed parameters and return types before the handler can be built.

**Primary recommendation:** Implement in two logical groups — discovery operations first (static/near-static XML from constants), then control operations (trait delegation with typed parameters). Build the Frigate compat test after both groups pass individually.

---

## Standard Stack

### Core (no new dependencies required)

| Library | Version | Purpose | Already Present |
|---------|---------|---------|----------------|
| `quick-xml` | 0.39 | Extract operation name and request parameters from SOAP body | Yes |
| `bytes` | 1 | `Bytes` in/out for `SoapHandler::handle` | Yes |
| `async-trait` | 0.1 | `#[async_trait]` on `PTZService` trait | Yes |
| `soap-server` | path dep | `SoapHandler`, `SoapFault`, `ServerBuilder` | Yes |
| `axum` | 0.8 | `Router::merge()` for third-service routing | Yes |

**No new dependencies required.** All libraries already present in Cargo.toml.

### New Rust Types Needed (hand-written stubs in `src/generated/types.rs`)

| Type | Key Fields | Source |
|------|-----------|--------|
| `PTZNode` | `token: String`, `name: String`, `supported_ptz_spaces: PTZSpaces`, `max_presets: i32`, `home_supported: bool` | onvif.xsd lines 4464-4522 |
| `PTZSpaces` | `relative_pan_tilt_translation_space: Vec<Space2DDescription>` (others optional) | onvif.xsd lines 4883-4965 |
| `Space2DDescription` | `uri: String`, `x_range: FloatRange`, `y_range: FloatRange` | onvif.xsd lines 4966-4990 |
| `FloatRange` | `min: f32`, `max: f32` | onvif.xsd (MinMax floats) |
| `PTZConfigurationData` | `token: String`, `name: String`, `node_token: String` | onvif.xsd lines 4567-4675 (simplified) |
| `PTZStatus` | `move_status: PTZMoveStatus` | common.xsd lines 98-138 |
| `PTZMoveStatus` | `pan_tilt: MoveStatus`, `zoom: MoveStatus` | common.xsd lines 139-154 |
| `MoveStatus` | enum: `Idle`, `Moving`, `Unknown` | common.xsd lines 155-161 |
| `PTZPreset` | `token: String`, `name: String` | onvif.xsd lines 5028-5052 |
| `PanTiltVec` | `x: f32`, `y: f32` | Used in RelativeMove/AbsoluteMove/ContinuousMove params |
| `ZoomVec` | `x: f32` | Used in Zoom component of move params |

Note: These are simplified Rust structs — they do not need all optional XSD fields. The handler renders XML directly from format strings. Only add fields the handler actually uses.

### Installation

```bash
# No new dependencies — all already present
cargo check -p onvif-server
```

---

## Architecture Patterns

### Recommended File Structure

```
src/
├── service/
│   ├── mod.rs          # MODIFY — add: pub mod ptz;
│   ├── device.rs       # existing — no changes
│   ├── media.rs        # existing — no changes
│   └── ptz.rs          # NEW — PTZServiceHandler (mirrors media.rs)
├── traits/
│   └── ptz.rs          # MODIFY — typed return signatures (currently all ())
├── generated/
│   └── types.rs        # MODIFY — add PTZ-specific types
├── server.rs           # MODIFY — wire PTZServiceHandler in run()
└── lib.rs              # MODIFY — pub use new types + PTZServiceHandler
tests/
└── frigate_compat.rs   # NEW — Frigate autotracker call sequence test (TEST-01)
examples/
└── virtual_ptz.rs      # NEW — minimal consumer example (TEST-02)
```

### Pattern 1: PTZServiceHandler mirrors MediaServiceHandler

**What:** One struct implementing `SoapHandler`, holding `Arc<dyn PTZService>`, dispatching by operation name via `extract_local_name + match`. Static responses for discovery ops, trait delegation for control ops.

```rust
// src/service/ptz.rs
use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};
use quick_xml::NsReader;
use quick_xml::events::Event;

use crate::error::OnvifError;
use crate::traits::PTZService;
use crate::constants::{PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV, PROFILE_TOKEN};

pub struct PTZServiceHandler {
    pub(crate) svc: Arc<dyn PTZService>,
}

#[async_trait]
impl SoapHandler for PTZServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetNodes"                 => self.handle_get_nodes().await,
            "GetNode"                  => self.handle_get_node(&body).await,
            "GetConfigurations"        => self.handle_get_configurations().await,
            "GetConfiguration"         => self.handle_get_configuration(&body).await,
            "GetConfigurationOptions"  => self.handle_get_configuration_options(&body).await,
            "GetServiceCapabilities"   => self.handle_get_service_capabilities().await,
            "RelativeMove"             => self.handle_relative_move(&body).await,
            "AbsoluteMove"             => self.handle_absolute_move(&body).await,
            "ContinuousMove"           => self.handle_continuous_move(&body).await,
            "Stop"                     => self.handle_stop(&body).await,
            "GetStatus"                => self.handle_get_status(&body).await,
            "GetPresets"               => self.handle_get_presets(&body).await,
            "GotoPreset"               => self.handle_goto_preset(&body).await,
            "SetPreset"                => self.handle_set_preset(&body).await,
            "RemovePreset"             => self.handle_remove_preset(&body).await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}
```

The PTZ WSDL namespace prefix is `tptz:` (confirmed from ptz.wsdl `targetNamespace="http://www.onvif.org/ver10/ptz/wsdl"`). The schema types use `tt:` as in other services.

### Pattern 2: GetNodes — static response with TranslationSpaceFov

**What:** Returns the single PTZ node with the `SupportedPTZSpaces` element containing `RelativePanTiltTranslationSpace` with the exact `TRANSLATION_SPACE_FOV` URI.

```rust
async fn handle_get_nodes(&self) -> Result<Bytes, SoapFault> {
    let xml = format!(
        r#"<tptz:GetNodesResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZNode token="{node_token}" FixedHomePosition="false">
    <tt:Name>PTZNode</tt:Name>
    <tt:SupportedPTZSpaces>
      <tt:RelativePanTiltTranslationSpace>
        <tt:URI>{fov_uri}</tt:URI>
        <tt:XRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:XRange>
        <tt:YRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:YRange>
      </tt:RelativePanTiltTranslationSpace>
    </tt:SupportedPTZSpaces>
    <tt:MaximumNumberOfPresets>10</tt:MaximumNumberOfPresets>
    <tt:HomeSupported>false</tt:HomeSupported>
  </tptz:PTZNode>
</tptz:GetNodesResponse>"#,
        node_token = PTZ_NODE_TOKEN,
        fov_uri = TRANSLATION_SPACE_FOV,
    );
    Ok(Bytes::from(xml))
}
```

Frigate checks `ptz_config.Spaces.RelativePanTiltTranslationSpace` (from GetConfigurationOptions) but zeep also parses GetNodes for node discovery. Include the URI in both.

### Pattern 3: GetConfigurationOptions — must include TranslationSpaceFov in Spaces

**What:** Frigate calls `GetConfigurationOptions` and runs:
```python
fov_space_id = next(
    (i for i, space in enumerate(
        ptz_config.Spaces.RelativePanTiltTranslationSpace
    ) if "TranslationSpaceFov" in space["URI"]),
    None,
)
```
If `fov_space_id` is `None`, relative FOV moves are disabled.

```rust
async fn handle_get_configuration_options(&self, _body: &Bytes) -> Result<Bytes, SoapFault> {
    let xml = format!(
        r#"<tptz:GetConfigurationOptionsResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZConfigurationOptions>
    <tt:Spaces>
      <tt:RelativePanTiltTranslationSpace>
        <tt:URI>{fov_uri}</tt:URI>
        <tt:XRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:XRange>
        <tt:YRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:YRange>
      </tt:RelativePanTiltTranslationSpace>
    </tt:Spaces>
    <tt:PTZTimeout><tt:Min>PT0S</tt:Min><tt:Max>PT60S</tt:Max></tt:PTZTimeout>
  </tptz:PTZConfigurationOptions>
</tptz:GetConfigurationOptionsResponse>"#,
        fov_uri = TRANSLATION_SPACE_FOV,
    );
    Ok(Bytes::from(xml))
}
```

### Pattern 4: GetServiceCapabilities — MoveStatus="true" as XML attribute

**What:** Frigate calls `find_by_key(vars(service_capabilities), "MoveStatus")`. The zeep library deserializes the `Capabilities` element and looks for a `MoveStatus` attribute on it. This MUST be an XML attribute (not a child element) on the `Capabilities` element.

```rust
async fn handle_get_service_capabilities(&self) -> Result<Bytes, SoapFault> {
    let xml = r#"<tptz:GetServiceCapabilitiesResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl">
  <tptz:Capabilities MoveStatus="true" StatusPosition="false"/>
</tptz:GetServiceCapabilitiesResponse>"#;
    Ok(Bytes::from(xml))
}
```

### Pattern 5: GetStatus — PTZStatus with nested MoveStatus

**What:** Frigate reads `status.MoveStatus.PanTilt` and `status.MoveStatus.Zoom`. The zeep library deserializes these from the XML element hierarchy. Both must be present and set to "IDLE" or "MOVING".

PTZStatus (common.xsd) → MoveStatus element (type PTZMoveStatus) → PanTilt element + Zoom element (both type MoveStatus simpleType: IDLE/MOVING/UNKNOWN).

```rust
async fn handle_get_status(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
    let profile_token = extract_text_element(body, "ProfileToken")?;
    let status = self.svc.get_status(&profile_token).await
        .map_err(|e| e.into_soap_fault())?;
    let pan_tilt = if status.is_moving { "MOVING" } else { "IDLE" };
    let zoom = if status.zoom_moving { "MOVING" } else { "IDLE" };
    let xml = format!(
        r#"<tptz:GetStatusResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZStatus>
    <tt:MoveStatus>
      <tt:PanTilt>{pan_tilt}</tt:PanTilt>
      <tt:Zoom>{zoom}</tt:Zoom>
    </tt:MoveStatus>
  </tptz:PTZStatus>
</tptz:GetStatusResponse>"#,
        pan_tilt = pan_tilt,
        zoom = zoom,
    );
    Ok(Bytes::from(xml))
}
```

Note: Frigate also has a fallback: `if pan_tilt_status is None: pan_tilt_status = getattr(status, "MoveStatus", None)`. This handles older cameras that return a flat MoveStatus string. The nested structure is the correct form per spec.

### Pattern 6: RelativeMove — parse PanTilt x/y from body

**What:** Frigate sets `Translation.PanTilt.x` and `Translation.PanTilt.y` on the move request. The handler must extract these floats from the XML body.

The XML body arrives as:
```xml
<tptz:RelativeMove>
  <tptz:ProfileToken>profile_0</tptz:ProfileToken>
  <tptz:Translation>
    <tt:PanTilt x="0.5" y="-0.3" space="http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov"/>
    <tt:Zoom x="0.0"/>
  </tptz:Translation>
  <tptz:Speed>
    <tt:PanTilt x="1.0" y="1.0"/>
  </tptz:Speed>
</tptz:RelativeMove>
```

The `x` and `y` values are XML attributes on the `<tt:PanTilt>` element (Vector2D type), not child elements. This requires attribute extraction, not text-element extraction. Use `quick_xml` attribute parsing:

```rust
// Parse PanTilt element attributes: x, y
// Parse Zoom element attribute: x
// Use NsReader to walk body, look for PanTilt inside Translation, extract x/y attrs
```

A helper `extract_vector2d_attrs(body, "PanTilt")` is needed to find the element and return its `x` and `y` attributes as `f32`.

### Pattern 7: PTZService trait — typed signatures

The current `PTZService` trait has all methods returning `Result<(), OnvifError>` with no parameters. Phase 4 must expand these to typed signatures. Operations split into two groups:

**Handler-internal (static/near-static — no trait delegation needed):**
- GetNodes, GetConfigurations, GetConfiguration, GetConfigurationOptions, GetServiceCapabilities

**Trait-delegated (consumer provides implementation):**
```rust
#[async_trait]
pub trait PTZService: Send + Sync + 'static {
    async fn relative_move(&self, profile_token: &str, pan: f32, tilt: f32, zoom: f32)
        -> Result<(), OnvifError> { not_implemented() }

    async fn absolute_move(&self, profile_token: &str, pan: f32, tilt: f32, zoom: f32)
        -> Result<(), OnvifError> { not_implemented() }

    async fn continuous_move(&self, profile_token: &str, pan: f32, tilt: f32, zoom: f32)
        -> Result<(), OnvifError> { not_implemented() }

    async fn stop(&self, profile_token: &str, pan_tilt: bool, zoom: bool)
        -> Result<(), OnvifError> { not_implemented() }

    async fn get_status(&self, profile_token: &str)
        -> Result<PTZStatusResult, OnvifError> { not_implemented() }

    async fn get_presets(&self, profile_token: &str)
        -> Result<Vec<PTZPreset>, OnvifError> { Ok(vec![]) }

    async fn goto_preset(&self, profile_token: &str, preset_token: &str)
        -> Result<(), OnvifError> { not_implemented() }

    async fn set_preset(&self, profile_token: &str, preset_name: Option<&str>, preset_token: Option<&str>)
        -> Result<String, OnvifError> { not_implemented() }

    async fn remove_preset(&self, profile_token: &str, preset_token: &str)
        -> Result<(), OnvifError> { not_implemented() }
}
```

`PTZStatusResult` is a small Rust struct returned from `get_status` — the handler serializes it to XML:
```rust
pub struct PTZStatusResult {
    pub pan_tilt_moving: bool,
    pub zoom_moving: bool,
}
```

### Pattern 8: Router::merge() for PTZ in run()

```rust
// In OnvifServer::run() — add third ServerBuilder block after media:
let ptz_svc = self.ptz_service
    .ok_or("ptz_service is required to call run()")?;

let ptz_handler = PTZServiceHandler { svc: ptz_svc };

let username3 = self.username.clone();
let password3 = self.password.clone();

let ptz_soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
        include_bytes!("../wsdl/ptz.wsdl").to_vec(),
        EmbeddedWsdlLoader,
    )
    .path("/onvif/ptz_service")
    .default_handler(ptz_handler)
    .auth(move |user: &str| -> Option<String> {
        if Some(user) == username3.as_deref() { password3.clone() } else { None }
    })
    .auth_bypass(std::iter::empty::<String>())
    .build()
    .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

let router = soap_svc.into_router()
    .merge(media_soap_svc.into_router())
    .merge(ptz_soap_svc.into_router());
```

The `PTZServiceHandler` does not need an `xaddr` field (unlike DeviceServiceHandler) — it serves PTZ operations only, not service discovery.

### Pattern 9: Frigate compat test structure

```rust
// tests/frigate_compat.rs
// Replays Frigate autotracker call sequence against handler stubs.
// Sequence per Frigate source (frigate/ptz/onvif.py _init_onvif + autotracking):
//   1. Media: GetProfiles → extract PTZConfiguration.token (ptz_cfg_0), PTZConfiguration.NodeToken (ptz_node_0)
//   2. PTZ: GetConfigurationOptions(ConfigurationToken=ptz_cfg_0) → check Spaces.RelativePanTiltTranslationSpace URI
//   3. PTZ: GetServiceCapabilities → check MoveStatus="true"
//   4. PTZ: GetPresets(ProfileToken=profile_0) → list presets
//   5. PTZ: GetStatus(ProfileToken=profile_0) → check MoveStatus.PanTilt ∈ {IDLE, MOVING}
//   6. PTZ: RelativeMove(ProfileToken=profile_0, Translation.PanTilt.x=0.5, y=0.3) → Ok
//   7. PTZ: GotoPreset(ProfileToken=profile_0, PresetToken=...) → Ok (if presets non-empty)

#[tokio::test]
async fn frigate_autotracker_call_sequence() { ... }
```

The test drives both `MediaServiceHandler` and `PTZServiceHandler` directly (no HTTP server) using `handler.handle(body).await`.

### Anti-Patterns to Avoid

- **Wrong WSDL namespace prefix:** PTZ uses `tptz:` (not `tds:` or `trt:`). From ptz.wsdl: `targetNamespace="http://www.onvif.org/ver10/ptz/wsdl"`.
- **MoveStatus as child element instead of XML attribute:** `GetServiceCapabilities` must return `<tptz:Capabilities MoveStatus="true"/>` — the MoveStatus is an XML *attribute* on `Capabilities`. Zeep's `vars()` only finds it if it is an attribute, not a child element.
- **Flat MoveStatus in GetStatus:** GetStatus must return the nested `<tt:MoveStatus><tt:PanTilt>IDLE</tt:PanTilt><tt:Zoom>IDLE</tt:Zoom></tt:MoveStatus>` structure (PTZMoveStatus type), not a flat `<tt:MoveStatus>IDLE</tt:MoveStatus>` string.
- **TranslationSpaceFov only in GetNodes, not GetConfigurationOptions:** Frigate reads from `GetConfigurationOptions`, not `GetNodes`. Both should include it, but GetConfigurationOptions is the critical one.
- **Using GetNode token as profile token:** `GetNode` expects a *node* token (PTZ_NODE_TOKEN); `RelativeMove`, `GetStatus`, etc. expect a *profile* token (PROFILE_TOKEN). These are different.
- **PanTilt values as child elements instead of XML attributes:** PTZVector/Vector2D in the ONVIF spec uses `x` and `y` as XML attributes on the element, not child elements. Frigate sends `<tt:PanTilt x="0.5" y="-0.3"/>`.
- **ptz_service optional in run() when always required:** For Frigate compatibility, ptz_service is a hard requirement. Require it in `run()` with a clear error, same as device_service and media_service.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| XML namespace handling | Custom namespace tracker | `quick_xml::NsReader` | Already proven in device.rs and media.rs |
| SOAP fault serialization | Custom fault builder | `OnvifError::into_soap_fault()` | Already defined with correct ter: namespace |
| ProfileToken/ConfigurationToken extraction | Manual byte scan | Reuse `extract_text_element` pattern from media.rs | NsReader handles namespace-prefixed elements |
| Vector2D attribute extraction | Regex/manual scan | New `extract_element_attrs` helper via NsReader | Must handle `x`/`y` as XML attributes not text |
| Float parsing | Custom parser | Rust `str::parse::<f32>()` | Infallible for well-formed ONVIF values |

**Key insight:** The format-string XML approach from Phases 2-3 is deliberate and correct for PTZ too. Discovery responses (GetNodes, GetConfigurations, GetConfigurationOptions) are fully static — built from constants, no trait calls needed. Control responses (RelativeMove, Stop, etc.) return empty response elements; only GetStatus, GetPresets, SetPreset return data.

---

## Common Pitfalls

### Pitfall 1: TranslationSpaceFov URI typo or missing from GetConfigurationOptions

**What goes wrong:** Frigate autotracker `fov_space_id` returns `None`, disabling RelativeMove. No error message.
**Why it happens:** The URI check is `"TranslationSpaceFov" in space["URI"]` — a substring match — but the URI must still be the correct one. A typo in the URI or omitting the space entirely silently breaks tracking.
**How to avoid:** Use `TRANSLATION_SPACE_FOV` constant (`http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov`) in the XML. Assert the exact string in the integration test.
**Warning signs:** Frigate PTZ autotracking does not activate; no error in logs; PTZ controls work manually.

### Pitfall 2: GetServiceCapabilities MoveStatus format

**What goes wrong:** Returning `<tptz:Capabilities><tptz:MoveStatus>true</tptz:MoveStatus></tptz:Capabilities>` (child element) instead of `<tptz:Capabilities MoveStatus="true"/>` (XML attribute). Frigate's `find_by_key(vars(capabilities), "MoveStatus")` only finds XML attributes when zeep deserializes the response.
**Why it happens:** It's natural to model boolean values as child elements in XML. ONVIF's service capabilities spec defines MoveStatus as an attribute on the Capabilities complex type.
**How to avoid:** Emit `<tptz:Capabilities MoveStatus="true"/>` — the MoveStatus value is an XML attribute, not a child element.
**Warning signs:** Frigate logs "Camera does not support GetServiceCapabilities" or autotracker never polls GetStatus.

### Pitfall 3: PTZStatus GetStatus nested vs flat structure

**What goes wrong:** Handler returns `<tt:MoveStatus>IDLE</tt:MoveStatus>` (flat simpleType) instead of the `PTZMoveStatus` complex type with `PanTilt` and `Zoom` children. Frigate reads `status.MoveStatus.PanTilt` and gets `None` if the nesting is absent.
**Why it happens:** The ONVIF XSD has both a `MoveStatus` simpleType (enum) and a `PTZMoveStatus` complexType (container). The PTZStatus type uses `PTZMoveStatus` (the container), not the simpleType directly.
**How to avoid:** The GetStatus response MoveStatus element must be the container: `<tt:MoveStatus><tt:PanTilt>IDLE</tt:PanTilt><tt:Zoom>IDLE</tt:Zoom></tt:MoveStatus>`.
**Warning signs:** Autotracker overshoots; Frigate logs show exception from `status.MoveStatus.PanTilt` not being `IDLE` or `MOVING`.

### Pitfall 4: RelativeMove PanTilt as child elements vs XML attributes

**What goes wrong:** Parsing `<Translation><PanTilt><x>0.5</x><y>0.3</y></PanTilt></Translation>` but the actual request uses `<Translation><PanTilt x="0.5" y="0.3"/>` (attributes). The handler extracts zeros or fails.
**Why it happens:** ONVIF's Vector2D type uses `x` and `y` as XML schema attributes (`xs:attribute`), not elements. The `extract_text_element` helper from media.rs only handles text content, not attributes.
**How to avoid:** Write a `extract_element_attribute(body, element_name, attr_name)` helper that uses NsReader to find the element and then reads its attribute. This is different from `extract_text_element`.
**Warning signs:** RelativeMove always sends zero pan/tilt to the trait; camera appears to ignore movement commands.

### Pitfall 5: Token confusion across operations

**What goes wrong:** A `RelativeMove` request arrives with `ProfileToken=profile_0`. The handler tries to look up a PTZ config for that token. A `GetNode` request arrives with `NodeToken=profile_0` (wrong — should be `ptz_node_0`). Different token spaces collide.
**Why it happens:** All operations use the same `token` terminology but reference different objects. Profile tokens come from GetProfiles; node tokens from GetNodes; config tokens from GetConfigurations.
**How to avoid:** Discovery operations (GetNodes, GetNode, GetConfigurations, GetConfiguration) validate against PTZ_NODE_TOKEN or PTZ_CONFIG_TOKEN. Control operations (RelativeMove, GetStatus, etc.) accept PROFILE_TOKEN and ignore/forward it to the trait. Define clearly: "control ops receive profile_token, pass it to trait as-is".
**Warning signs:** GetNode returns NotFound when called with a profile token; RelativeMove fails when called with a config token.

### Pitfall 6: auth closure ownership in third ServerBuilder block

**What goes wrong:** Compile error — `username` and `password` already moved into the second (media) closure.
**Why it happens:** Rust moves values into closures. Phase 3 already required cloning for the media block. Phase 4 needs a third clone.
**How to avoid:** Clone before the media block: `username2`, `username3`, `password2`, `password3`. Each closure captures its own owned copy.

### Pitfall 7: ptz.wsdl operation name for Stop

**What goes wrong:** The dispatch table uses `"Stop"` but the WSDL defines the element as `StopRequest` (as seen in ptz.wsdl line 279: `<xs:element name="StopRequest">`). The `extract_local_name` returns the body element name, which is what the client sends.
**Why it happens:** The WSDL element for the stop request is named `StopRequest` in the schema types section but the WSDL message/operation name is `Stop`. ONVIF clients send `<tptz:Stop>` (the operation name), not `<tptz:StopRequest>`.
**How to avoid:** Match on `"Stop"` in the dispatch table — this is the operation local name that clients send. The `StopRequest` naming in the XSD types section is the schema element name, not the on-wire element name.
**Verification:** Check ptz.wsdl message definitions — `<wsdl:message name="StopRequest">` uses element `tptz:StopRequest`. But clients send the SOAP body element matching the WSDL operation input message element name. The operation is named `Stop` in the portType, so the input message element on the wire is `<tptz:Stop>`. Confirm by checking what name Frigate or ONVIF DM actually sends.

---

## Code Examples

### GetNodes response (verified from onvif.xsd PTZNode type + PTZSpaces type)

```xml
<tptz:GetNodesResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZNode token="ptz_node_0" FixedHomePosition="false">
    <tt:Name>PTZNode</tt:Name>
    <tt:SupportedPTZSpaces>
      <tt:RelativePanTiltTranslationSpace>
        <tt:URI>http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov</tt:URI>
        <tt:XRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:XRange>
        <tt:YRange><tt:Min>-1</tt:Min><tt:Max>1</tt:Max></tt:YRange>
      </tt:RelativePanTiltTranslationSpace>
    </tt:SupportedPTZSpaces>
    <tt:MaximumNumberOfPresets>10</tt:MaximumNumberOfPresets>
    <tt:HomeSupported>false</tt:HomeSupported>
  </tptz:PTZNode>
</tptz:GetNodesResponse>
```

Source: onvif.xsd lines 4464-4522 (PTZNode), 4883-4965 (PTZSpaces), 4966-4990 (Space2DDescription)

### GetConfigurations response (verified from onvif.xsd PTZConfiguration type)

```xml
<tptz:GetConfigurationsResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZConfigurations token="ptz_cfg_0">
    <tt:Name>PTZConfig</tt:Name>
    <tt:UseCount>1</tt:UseCount>
    <tt:NodeToken>ptz_node_0</tt:NodeToken>
    <tt:DefaultContinuousPanTiltVelocitySpace>http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov</tt:DefaultContinuousPanTiltVelocitySpace>
  </tptz:PTZConfigurations>
</tptz:GetConfigurationsResponse>
```

Source: onvif.xsd lines 4567-4675 (PTZConfiguration). The `token` attribute is on the outer element; `NodeToken` is a required child element.

### GetStatus response (verified from common.xsd PTZStatus + PTZMoveStatus types)

```xml
<tptz:GetStatusResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZStatus>
    <tt:MoveStatus>
      <tt:PanTilt>IDLE</tt:PanTilt>
      <tt:Zoom>IDLE</tt:Zoom>
    </tt:MoveStatus>
  </tptz:PTZStatus>
</tptz:GetStatusResponse>
```

Source: common.xsd lines 98-161 (PTZStatus, PTZMoveStatus, MoveStatus enum)

### GetPresets response (verified from onvif.xsd PTZPreset type)

```xml
<tptz:GetPresetsResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tptz:PTZPreset token="preset_0">
    <tt:Name>Home</tt:Name>
  </tptz:PTZPreset>
</tptz:GetPresetsResponse>
```

Empty response when no presets: `<tptz:GetPresetsResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>`. Source: onvif.xsd lines 5028-5052 (PTZPreset).

### Empty response elements (RelativeMove, ContinuousMove, AbsoluteMove, Stop, GotoPreset)

```xml
<tptz:RelativeMoveResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>
<tptz:ContinuousMoveResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>
<tptz:AbsoluteMoveResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>
<tptz:StopResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>
<tptz:GotoPresetResponse xmlns:tptz="http://www.onvif.org/ver10/ptz/wsdl"/>
```

Source: ptz.wsdl — all these response types have empty `<xs:complexType><xs:sequence/></xs:complexType>`.

### extract_element_attribute helper (new — for RelativeMove PanTilt x/y)

```rust
/// Extract a named attribute from the first occurrence of a named element.
/// Used for Vector2D types (PanTilt, Zoom) where x/y are XML attributes.
fn extract_element_attribute(
    body: &Bytes,
    element_name: &str,
    attr_name: &str,
) -> Result<f32, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_resolved_event().map_err(|e| SoapFault::sender(format!("{e}")))? {
            (_, Event::Start(e)) | (_, Event::Empty(e)) => {
                let local = std::str::from_utf8(e.local_name().as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?;
                if local == element_name {
                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.local_name().as_ref())
                            .map_err(|e| SoapFault::sender(format!("{e}")))?;
                        if key == attr_name {
                            let val = std::str::from_utf8(attr.value.as_ref())
                                .map_err(|e| SoapFault::sender(format!("{e}")))?;
                            return val.parse::<f32>()
                                .map_err(|e| SoapFault::sender(format!("{e}")));
                        }
                    }
                }
            }
            (_, Event::Eof) => return Ok(0.0), // element not found — default 0
            _ => {}
        }
    }
}
```

### Frigate call sequence for tests/frigate_compat.rs

Frigate's `_init_onvif` sequence (verified from frigate/ptz/onvif.py):
1. Media `GetProfiles` → verify `PTZConfiguration.token = ptz_cfg_0`, `PTZConfiguration.NodeToken = ptz_node_0`
2. PTZ `GetConfigurationOptions(ConfigurationToken=ptz_cfg_0)` → verify `Spaces.RelativePanTiltTranslationSpace[0].URI` contains "TranslationSpaceFov"
3. PTZ `GetServiceCapabilities()` → verify response XML contains `MoveStatus="true"` attribute
4. PTZ `GetPresets(ProfileToken=profile_0)` → list (may be empty)
5. PTZ `GetStatus(ProfileToken=profile_0)` → verify MoveStatus.PanTilt ∈ {IDLE, MOVING}
6. PTZ `RelativeMove(ProfileToken=profile_0, Translation.PanTilt x=0.5 y=0.3)` → Ok(())
7. PTZ `GotoPreset(ProfileToken=profile_0, PresetToken=...)` → Ok(()) (if preset exists)

---

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|------------------|-------|
| PTZService trait returning `()` | PTZService trait with typed params (f32 pan/tilt/zoom, bool stop flags) | Phase 4 expands from stub signatures |
| Single service server | Triple-merged router (device + media + ptz) | Same Router::merge() pattern |
| No PTZ test coverage | frigate_compat.rs end-to-end test | Critical for verifying Frigate compatibility |
| No consumer example | virtual_ptz.rs example | Documents the full API surface |

---

## Open Questions

1. **Stop operation element name on the wire**
   - What we know: ptz.wsdl XSD types section defines `<xs:element name="StopRequest">`, but the WSDL portType operation is named `Stop`. The WSDL message `<wsdl:message name="StopRequest">` wraps the `tptz:StopRequest` element.
   - What's unclear: Does the SOAP body element sent by Frigate/zeep use `<tptz:Stop>` or `<tptz:StopRequest>`? The operation name in the WSDL binding is the key.
   - Recommendation: Check ptz.wsdl `portType` and `binding` sections for the operation input element name. If the input element is `StopRequest`, the dispatch match arm should be `"StopRequest"`. Most ONVIF operations match operation name = element name but this one differs in the XSD types. Verify before coding.

2. **GetConfiguration vs GetNode token scoping**
   - What we know: GetNode uses `NodeToken`; GetConfiguration uses `ConfigurationToken`. Both the server and client reference the same single node/config.
   - What's unclear: Whether Frigate ever calls GetNode or GetConfiguration (vs. just GetConfigurationOptions). If not called in the test sequence, stubs returning the static single node/config with no token validation are sufficient.
   - Recommendation: Implement static responses returning the single PTZ node/config regardless of the requested token — return not_found only if the token is non-empty AND doesn't match the known constant. This is the minimal correct implementation.

3. **virtual_ptz example: ptz_service optional or required in run()**
   - What we know: The current pattern makes each service required at `run()` time. The example must call `.ptz_service(...)` on the builder.
   - What's unclear: Should the example also demonstrate Device + Media + PTZ all registered, or can it skip Device/Media? For Frigate use, all three must be registered.
   - Recommendation: The virtual_ptz example registers all three services (Device + Media + PTZ) and sets up a minimal but complete in-memory implementation. This demonstrates the actual consumer pattern.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | tokio::test (no external test framework — existing pattern) |
| Config file | none — `[dev-dependencies]` in Cargo.toml |
| Quick run command | `cargo test -p onvif-server ptz` |
| Full suite command | `cargo test -p onvif-server` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PTZ-01 | GetNodes returns TranslationSpaceFov URI in RelativePanTiltTranslationSpace | unit | `cargo test -p onvif-server ptz_get_nodes` | ❌ Wave 0 |
| PTZ-02 | GetNode with node token returns same node data | unit | `cargo test -p onvif-server ptz_get_node` | ❌ Wave 0 |
| PTZ-03 | GetConfigurations returns config with NodeToken=ptz_node_0 | unit | `cargo test -p onvif-server ptz_get_configurations` | ❌ Wave 0 |
| PTZ-04 | GetConfiguration with config token returns config | unit | `cargo test -p onvif-server ptz_get_configuration` | ❌ Wave 0 |
| PTZ-05 | GetConfigurationOptions Spaces.RelativePanTiltTranslationSpace URI contains TranslationSpaceFov | unit | `cargo test -p onvif-server ptz_get_configuration_options` | ❌ Wave 0 |
| PTZ-06 | GetServiceCapabilities XML contains MoveStatus="true" attribute | unit | `cargo test -p onvif-server ptz_get_service_capabilities` | ❌ Wave 0 |
| PTZ-07 | RelativeMove invokes trait with correct pan/tilt floats extracted from body | unit | `cargo test -p onvif-server ptz_relative_move` | ❌ Wave 0 |
| PTZ-08 | ContinuousMove invokes trait with velocity params | unit | `cargo test -p onvif-server ptz_continuous_move` | ❌ Wave 0 |
| PTZ-09 | Stop invokes trait with PanTilt=true, Zoom=true booleans | unit | `cargo test -p onvif-server ptz_stop` | ❌ Wave 0 |
| PTZ-10 | GetStatus returns MoveStatus.PanTilt and .Zoom elements | unit | `cargo test -p onvif-server ptz_get_status` | ❌ Wave 0 |
| PTZ-11 | GetPresets returns consumer's preset list | unit | `cargo test -p onvif-server ptz_get_presets` | ❌ Wave 0 |
| PTZ-12 | GotoPreset invokes trait with preset token | unit | `cargo test -p onvif-server ptz_goto_preset` | ❌ Wave 0 |
| PTZ-13 | AbsoluteMove invokes trait with position params | unit | `cargo test -p onvif-server ptz_absolute_move` | ❌ Wave 0 |
| PTZ-14 | SetPreset invokes trait and returns preset token in response | unit | `cargo test -p onvif-server ptz_set_preset` | ❌ Wave 0 |
| PTZ-15 | RemovePreset invokes trait | unit | `cargo test -p onvif-server ptz_remove_preset` | ❌ Wave 0 |
| TEST-01 | Full Frigate autotracker sequence end-to-end | integration | `cargo test -p onvif-server frigate_autotracker_call_sequence` | ❌ Wave 0 |
| TEST-02 | virtual_ptz example compiles and starts | smoke | `cargo build --example virtual_ptz` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p onvif-server`
- **Per wave merge:** `cargo test -p onvif-server`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `tests/ptz_service.rs` — unit tests for PTZ-01 through PTZ-15
- [ ] `tests/frigate_compat.rs` — Frigate call sequence integration test (TEST-01)
- [ ] `examples/virtual_ptz.rs` — example binary (TEST-02)
- [ ] `src/service/ptz.rs` — PTZServiceHandler (new file)
- [ ] `PTZStatusResult`, `PTZPreset`, and supporting types in `src/generated/types.rs`

*(All Wave 0 gaps are new files — existing test infrastructure (device_management.rs, media_service.rs) remains unchanged)*

---

## Sources

### Primary (HIGH confidence)

- `src/service/media.rs` (repo) — MediaServiceHandler pattern: extract_local_name, match dispatch, format-string XML, extract_text_element
- `src/service/device.rs` (repo) — DeviceServiceHandler pattern reference
- `src/server.rs` (repo) — Router::merge() pattern, ServerBuilder wiring, auth closure clone pattern
- `src/constants.rs` (repo) — PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV confirmed present
- `src/traits/ptz.rs` (repo) — Current stub signatures confirmed; all return `()` with no params
- `wsdl/ptz.wsdl` (repo) — All 15 PTZ operation request/response element definitions confirmed
- `wsdl/onvif.xsd` (repo) — PTZNode (lines 4464-4522), PTZSpaces (4883-4965), Space2DDescription (4966-4990), PTZConfigurationOptions (4758-4792), PTZPreset (5028-5052), PTZConfiguration (4567-4675)
- `wsdl/common.xsd` (repo) — PTZStatus (lines 98-138), PTZMoveStatus (139-154), MoveStatus enum IDLE/MOVING/UNKNOWN (155-161)
- `frigate/ptz/onvif.py` (GitHub, verified via WebFetch) — Exact Frigate call sequence; TranslationSpaceFov substring check; GetServiceCapabilities MoveStatus attribute check; GetStatus MoveStatus.PanTilt/Zoom parsing

### Secondary (MEDIUM confidence)

- `.planning/research/PITFALLS.md` (repo) — Pitfalls 3, 4, 5, 6 specifically for PTZ service
- `.planning/research/SUMMARY.md` (repo) — Phase 4 rationale and critical pitfalls summary
- `.planning/phases/03-media-service/03-RESEARCH.md` (repo) — Patterns 1-6 confirmed as established for reuse

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies; all libraries verified present in Cargo.toml
- Architecture: HIGH — confirmed from actual source files (device.rs, media.rs, server.rs); PTZ mirrors same pattern exactly
- XML type structures: HIGH — verified directly from bundled onvif.xsd and common.xsd; PTZ WSDL operation list confirmed from ptz.wsdl
- Frigate call sequence: HIGH — verified from Frigate source via WebFetch; Python code snippets confirm exact field names and checks
- Pitfalls: HIGH — derived from XSD analysis (attribute vs element distinction), Frigate source code review, and prior phase research

**Research date:** 2026-04-05
**Valid until:** 2026-05-05 (stable ONVIF spec and Frigate PTZ source)
