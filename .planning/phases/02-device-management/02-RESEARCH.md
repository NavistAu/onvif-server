# Phase 2: Device Management - Research

**Researched:** 2026-04-05
**Domain:** ONVIF Device Management Service — XML dispatch, request/response types, OnvifServer::run() wiring
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Porting operation following best prior art (onvif-rs, python-onvif-zeep, ONVIF spec) — same as Phase 1
- Follow established patterns from the research — don't reinvent where prior art already defines the right answer

### Claude's Discretion
- ServiceRouter pattern: How to bridge soap-server's `SoapHandler` (bytes in/out) to typed trait methods — deserialize XML request, call trait, serialize XML response
- XML serialization strategy: Whether to use yaserde, quick-xml manual, or string templates for ONVIF response XML
- DeviceServiceHandler implementation: Wire each ONVIF operation name to the corresponding `DeviceService` trait method
- OnvifServer.run() implementation: Wire builder fields into soap-server's `ServerBuilder`, bind port, start listener
- GetCapabilities vs GetServices response structure: Both must return correct XAddrs per research pitfall #10
- Request/response types: Expand hand-written stubs from Phase 1 to cover all Device Management types needed
- All technical decisions (dispatch pattern, XML format, type expansions) follow research and DESIGN.md

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DEV-01 | GetSystemDateAndTime without auth returns current UTC time and timezone | Handler uses `chrono::Utc::now()`; maps to `tt:SystemDateTime` XML; auth_bypass already pre-registered in builder |
| DEV-02 | GetCapabilities returns XAddrs for all registered services | `tt:Capabilities` has Device/Media/Events/PTZ/Imaging children each with XAddr; XAddr must mirror the request's bound address |
| DEV-03 | GetServices returns service namespace, XAddr, and capabilities for all registered services | `tds:GetServicesResponse` has `Service` elements with Namespace, XAddr, Version; IncludeCapability controls inline caps |
| DEV-04 | GetDeviceInformation returns manufacturer, model, firmware, serial, hardware ID from consumer config | `DeviceInfo` stub already in `generated/types.rs`; DeviceService trait needs typed return; response elements are literal xs:string |
| DEV-05 | GetScopes returns ONVIF-standard scope URIs | `tt:Scope` has ScopeDef (Fixed/Configurable) + ScopeItem (anyURI); standard scopes: `onvif://www.onvif.org/type/video_encoder` etc. |
| DEV-06 | GetHostname returns device hostname | `tt:HostnameInformation` has FromDHCP (bool) + Name (optional token); use `hostname::get()` or configured name |
| DEV-07 | GetNetworkInterfaces returns network interface info | `tt:NetworkInterface` extends `tt:DeviceEntity` (token attr); needs Enabled + optional Info/Link/IPv4; use `if-addrs` or hand-craft a minimal response |
</phase_requirements>

---

## Summary

Phase 2 implements the first end-to-end ONVIF service — Device Management — by wiring the Phase 1 scaffolding into a live HTTP server. Three work streams converge: (1) `OnvifServer::run()` must wire `OnvifServerBuilder` fields into soap-server's `ServerBuilder`, then spawn a tokio listener; (2) a `DeviceServiceHandler` struct must implement `SoapHandler`, parsing incoming XML bytes and dispatching to `DeviceService` trait methods; (3) hand-written response types must be expanded to cover all seven operations' response XML.

The critical architectural decision is the dispatch strategy. The devicemgmt.wsdl has **no `wsdl:service` element** and therefore no address declaration. This means soap-server falls into single-service mode with a configurable mount path (default `/soap`). Because the WSDL defines ~60 operations but Phase 2 implements only 7, the `ServerBuilder` must use `default_handler` to absorb the 50+ unimplemented operations with a `NotImplemented` SOAP fault — otherwise `build()` will return `BuildError::UnregisteredOperation` for each unregistered operation.

XML serialization uses quick-xml manual string building with `format!` — no external codegen required. This matches the Phase 1 decision to defer yaserde/xsd-parser (Rust 1.85.1 constraint). The trait methods need updated signatures returning typed response structs instead of `Result<(), OnvifError>`.

**Primary recommendation:** Implement in this order: (1) expand DeviceService trait method signatures to typed returns, (2) define hand-written response types, (3) implement DeviceServiceHandler as SoapHandler, (4) wire OnvifServer::run(), (5) add integration tests using axum-test or raw HTTP.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `soap-server` | path dep | `ServerBuilder`, `SoapService`, `SoapHandler`, `FnHandler`, auth dispatch | All SOAP infrastructure — confirmed from source; `into_router()` returns `axum::Router` |
| `axum` | `"0.8"` (via soap-server) | HTTP listener; `axum::serve` + `tokio::net::TcpListener` | OnvifServer::run() must drive the router; axum 0.8 is what soap-server already uses |
| `tokio` | `"1"` features `["full"]` | Async runtime, TCP binding | OnvifServer::run() is async; TcpListener::bind is tokio |
| `chrono` | `"0.4"` | `Utc::now()` for GetSystemDateAndTime | Already in Cargo.toml; provides Date/Time decomposition needed for tt:DateTime XML |
| `quick-xml` | `"0.39"` | Writing ONVIF response XML | Already in Cargo.toml; chosen over yaserde due to Rust 1.85.1 icu_* constraint |
| `bytes` | `"1"` | `Bytes` input/output for SoapHandler | SoapHandler contract: `async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault>` |
| `async-trait` | `"0.1"` | Async fn in DeviceServiceHandler SoapHandler impl | SoapHandler is an async trait; DeviceServiceHandler must implement it |

### No New Dependencies Required
Phase 2 adds no new Cargo.toml entries. All required crates are already declared in Phase 1.

**Exception:** `tokio` in `[dependencies]` currently has features `["sync"]` only. Phase 2 needs `["rt", "net"]` for `TcpListener` and the async runtime. Update the feature set.

Updated tokio dep:
```toml
tokio = { version = "1", features = ["rt", "net", "sync"] }
```

For dev-dependencies (integration tests), tokio already has `["full"]`.

---

## Architecture Patterns

### Recommended Module Structure
```
src/
├── server.rs           # OnvifServer + run() implementation — PRIMARY change in this phase
├── service/
│   └── device.rs       # DeviceServiceHandler — SoapHandler impl wrapping Arc<dyn DeviceService>
├── generated/
│   └── types.rs        # Expand: add DeviceServiceResponse types
├── traits/
│   └── device.rs       # Update method signatures to typed returns
├── error.rs            # No changes
├── constants.rs        # No changes
├── wsdl_loader.rs      # No changes
└── lib.rs              # Re-export DeviceServiceHandler if needed
```

### Pattern 1: DeviceServiceHandler as SoapHandler

The handler receives raw XML bytes of the body first-child element, determines the operation by element local name, delegates to the appropriate `DeviceService` trait method, and serializes the response.

```rust
// src/service/device.rs
use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};
use crate::traits::DeviceService;

pub struct DeviceServiceHandler {
    pub(crate) svc: Arc<dyn DeviceService>,
    pub(crate) xaddr: String,  // e.g. "http://192.168.1.10:8080/onvif/device_service"
}

#[async_trait]
impl SoapHandler for DeviceServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        // 1. Extract local element name from body bytes
        let op = extract_local_name(&body)?;
        // 2. Dispatch to trait method
        match op.as_str() {
            "GetSystemDateAndTime" => self.handle_get_system_date_and_time().await,
            "GetCapabilities"      => self.handle_get_capabilities().await,
            "GetServices"          => self.handle_get_services(&body).await,
            "GetDeviceInformation" => self.handle_get_device_information().await,
            "GetScopes"            => self.handle_get_scopes().await,
            "GetHostname"          => self.handle_get_hostname().await,
            "GetNetworkInterfaces" => self.handle_get_network_interfaces().await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}
```

**Key insight:** soap-server calls `handle()` with `body: Bytes` containing only the first child of the SOAP Body (not the full envelope), with all ancestor namespace declarations re-emitted on the root. Use `quick_xml::NsReader` to extract the local name.

### Pattern 2: Local Name Extraction from Body Bytes

```rust
fn extract_local_name(body: &Bytes) -> Result<String, SoapFault> {
    use quick_xml::NsReader;
    use quick_xml::events::Event;

    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_resolved_event().map_err(|e| SoapFault::sender(format!("{e}")))? {
            (_, Event::Start(e)) | (_, Event::Empty(e)) => {
                let local = std::str::from_utf8(e.local_name().as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?
                    .to_string();
                return Ok(local);
            }
            (_, Event::Eof) => return Err(SoapFault::sender("Empty body")),
            _ => {}
        }
    }
}
```

This is identical to the pattern used inside soap-server's `server.rs` for operation routing (`extract_body_qname`). High confidence.

### Pattern 3: OnvifServer::run() Wiring

`SoapService::into_router()` returns `axum::Router`. The caller drives it:

```rust
// src/server.rs — OnvifServer::run()
pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::serve;
    use tokio::net::TcpListener;

    let device_svc = self.device_service
        .ok_or("device_service is required")?;

    let xaddr = format!("http://0.0.0.0:{}/onvif/device_service", self.port);

    let handler = DeviceServiceHandler {
        svc: device_svc,
        xaddr: xaddr.clone(),
    };

    let username = self.username.clone();
    let password = self.password.clone();

    let soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
            include_bytes!("../wsdl/devicemgmt.wsdl").to_vec(),
            crate::EmbeddedWsdlLoader,
        )
        .path("/onvif/device_service")
        .handler("GetSystemDateAndTime", handler.clone())    // OR use default_handler
        // ... register each implemented operation ...
        .default_handler(UnimplementedHandler)               // absorbs ~50 unregistered ops
        .auth(move |user| {
            if Some(user) == username.as_deref() { password.clone() } else { None }
        })
        .auth_bypass(self.auth_bypass)
        .build()?;

    let router = soap_svc.into_router();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
    serve(listener, router).await?;
    Ok(())
}
```

**Critical constraint:** `DeviceServiceHandler` must be `Clone` (or the operations must be registered as separate `FnHandler` closures capturing an `Arc<DeviceServiceHandler>`). The `handler()` API on `ServerBuilder` accepts `impl SoapHandler` per call — one handler instance per operation name.

**Recommended pattern:** Register a single `Arc<DeviceServiceHandler>` via `default_handler` and have it handle all operations internally. This avoids per-operation handler registration and the Clone requirement:

```rust
let dh = Arc::new(DeviceServiceHandler { svc: device_svc, xaddr });
// Use as default_handler — handles all recognized ops, returns NotImplemented for others
.default_handler(dh)
// DO NOT register individual handlers — default_handler absorbs everything
```

This is cleaner: one handler instance, no clone needed (Arc satisfies SoapHandler bounds), no per-operation registration explosion.

### Pattern 4: Response XML Format

All responses must use the `tds:` prefix for the Device Management namespace. The WSDL defines:
- targetNamespace: `http://www.onvif.org/ver10/device/wsdl` → prefix `tds:`
- `tt:` types come from `http://www.onvif.org/ver10/schema`

Response bytes are the inner Body element (no envelope — soap-server wraps it):

```rust
fn get_system_date_and_time_response(now: chrono::DateTime<chrono::Utc>) -> Bytes {
    let xml = format!(
        r#"<tds:GetSystemDateAndTimeResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:SystemDateAndTime>
    <tt:DateTimeType>Manual</tt:DateTimeType>
    <tt:DaylightSavings>false</tt:DaylightSavings>
    <tt:TimeZone><tt:TZ>UTC</tt:TZ></tt:TimeZone>
    <tt:UTCDateTime>
      <tt:Time><tt:Hour>{}</tt:Hour><tt:Minute>{}</tt:Minute><tt:Second>{}</tt:Second></tt:Time>
      <tt:Date><tt:Year>{}</tt:Year><tt:Month>{}</tt:Month><tt:Day>{}</tt:Day></tt:Date>
    </tt:UTCDateTime>
  </tds:SystemDateAndTime>
</tds:GetSystemDateAndTimeResponse>"#,
        now.hour(), now.minute(), now.second(),
        now.year(), now.month(), now.day()
    );
    Bytes::from(xml)
}
```

### Pattern 5: DeviceService Trait Method Signatures

Current stubs return `Result<(), OnvifError>`. Phase 2 must give each method a typed return:

```rust
#[async_trait]
pub trait DeviceService: Send + Sync + 'static {
    async fn get_system_date_and_time(&self) -> Result<chrono::DateTime<chrono::Utc>, OnvifError> {
        Ok(chrono::Utc::now())  // sensible default — no implementation needed from consumer
    }

    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        not_implemented()
    }

    async fn get_scopes(&self) -> Result<Vec<Scope>, OnvifError> {
        Ok(vec![
            Scope { scope_def: ScopeDefinition::Fixed, scope_item: "onvif://www.onvif.org/type/video_encoder".into() },
            Scope { scope_def: ScopeDefinition::Fixed, scope_item: "onvif://www.onvif.org/Profile/Streaming".into() },
        ])
    }

    async fn get_hostname(&self) -> Result<HostnameInformation, OnvifError> {
        Ok(HostnameInformation { from_dhcp: false, name: Some("onvif-device".into()) })
    }

    async fn get_network_interfaces(&self) -> Result<Vec<NetworkInterface>, OnvifError> {
        not_implemented()
    }
    // GetCapabilities + GetServices handled internally by the handler (no consumer override needed)
}
```

`GetCapabilities` and `GetServices` do not need consumer trait methods — the handler constructs them from the registered service list and bound address. These are framework-level responses.

### Anti-Patterns to Avoid

- **Registering 60 handlers individually:** The WSDL has ~60 operations. Do not register them one by one. Use `default_handler` and dispatch internally.
- **Returning full SOAP envelope in handler:** `SoapHandler::handle()` must return only the Body child XML, not a full envelope. Soap-server wraps it.
- **Hardcoding XAddrs as `localhost`:** XAddrs in GetCapabilities and GetServices must reflect the actual bound address. Pass the bound host/port through to the handler at construction time.
- **Not declaring namespace prefixes in response XML:** Each response element must declare `xmlns:tds` and `xmlns:tt` — soap-server does not inject them.
- **Omitting the `<tt:TZ>` child in TimeZone:** ONVIF clients (especially Frigate) parse `tt:TimeZone/tt:TZ` with a POSIX string like "UTC" or "GMT+0".

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SOAP envelope parsing | Custom envelope parser | soap-server — already done | Soap-server parses envelope, extracts body element, handles namespace re-emission |
| WS-Security auth | Custom digest validator | `soap_server::validate_username_token` + `ServerBuilder::auth()` | Nonce cache, timestamp tolerance, digest algorithm — complex and security-sensitive |
| HTTP server | Raw hyper or TCP | `soap_svc.into_router()` + `axum::serve` | Soap-server produces a composable axum Router; just serve it |
| WSDL serving | Custom WSDL HTTP route | Built into soap-server — GET ?wsdl handled automatically | `into_router()` registers the GET handler; EmbeddedWsdlLoader already wired in Phase 1 |
| Auth bypass logic | Custom middleware | `ServerBuilder::auth_bypass()` + builder's `HashSet<String>` | Already implemented in soap-server dispatch table per operation |

---

## Common Pitfalls

### Pitfall 1: `BuildError::UnregisteredOperation` for ~50 unimplemented ops
**What goes wrong:** Calling `ServerBuilder::build()` without a `default_handler` when the WSDL has operations without registered handlers returns `BuildError::UnregisteredOperation` on the first unregistered op name.
**Why it happens:** soap-server enforces that every WSDL operation is handled unless a `default_handler` is present. devicemgmt.wsdl has ~60 operations; Phase 2 implements 7.
**How to avoid:** Always call `.default_handler(unimplemented_handler)` before `.build()`. The `unimplemented_handler` returns `OnvifError::ActionNotSupported.into_soap_fault()`.
**Warning signs:** `build()` returns `Err(BuildError::UnregisteredOperation("SetSystemDateAndTime"))` or similar.

### Pitfall 2: XAddr uses localhost instead of bound address
**What goes wrong:** `GetCapabilities` and `GetServices` return XAddrs like `http://localhost:8080/onvif/device_service`. ONVIF clients on other hosts cannot connect using this address.
**Why it happens:** The handler constructs XAddrs from the port number alone without knowing the host the client used to reach the server.
**How to avoid:** Accept a `host` parameter in `DeviceServiceHandler` (e.g., `"0.0.0.0"` or a configured hostname). The ONVIF spec says XAddr scheme and IP must match the request — this is partially a client problem, but hardcoding `localhost` will break real clients.
**Recommended approach:** Accept `xaddr_base: String` at handler construction time (e.g., `"http://192.168.1.10:8080"`). Consumer sets it via a builder method `OnvifServerBuilder::advertised_host("192.168.1.10")`.

### Pitfall 3: Namespace declarations missing from response XML
**What goes wrong:** Response XML is valid structurally but ONVIF client fails to parse elements because the `tds:` or `tt:` prefix has no namespace binding.
**Why it happens:** soap-server wraps the response bytes in a SOAP envelope but does not inject service-specific namespace bindings.
**How to avoid:** Every response element that uses a prefix must declare its namespace inline or on the root response element: `xmlns:tds="http://www.onvif.org/ver10/device/wsdl"` and `xmlns:tt="http://www.onvif.org/ver10/schema"`.

### Pitfall 4: DeviceService trait `not_implemented()` return type mismatch
**What goes wrong:** Current trait stubs return `Result<(), OnvifError>`. Changing to typed returns (e.g., `Result<DeviceInfo, OnvifError>`) is a breaking change to the trait.
**Why it happens:** Phase 1 used `()` as a placeholder. Phase 2 must change all method signatures before implementing handlers.
**How to avoid:** Update ALL method signatures in the same plan step before writing any handler code that calls them.

### Pitfall 5: GetSystemDateAndTime auth bypass not passed to ServerBuilder
**What goes wrong:** GetSystemDateAndTime requires authentication when called without a Security header, returning a 401/fault. The builder pre-populates `auth_bypass` but it must be explicitly passed to `ServerBuilder::auth_bypass()`.
**Why it happens:** The `auth_bypass` HashSet on `OnvifServer` is populated in Phase 1, but `OnvifServer::run()` (Phase 2) is responsible for actually wiring it to soap-server.
**How to avoid:** In `run()`, call `.auth_bypass(self.auth_bypass.iter().cloned())` on the `ServerBuilder`.

### Pitfall 6: DeviceInfo fields not configurable without a DeviceService impl
**What goes wrong:** `GetDeviceInformation` returns empty strings if `DeviceService::get_device_information()` returns `not_implemented()` by default.
**Why it happens:** There is no builder-level storage for DeviceInfo fields in Phase 1.
**How to avoid:** Add `device_info: Option<DeviceInfo>` field to `OnvifServerBuilder` and expose a `.device_info(DeviceInfo)` builder method. The `DeviceService` default implementation uses this stored value, avoiding the need for the consumer to implement the trait just for device info.
**Alternative:** Keep it trait-only and document that consumers must implement `get_device_information()`.

### Pitfall 7: `tokio` dep missing `rt` and `net` features
**What goes wrong:** `tokio::net::TcpListener` is not available — compile error in `run()`.
**Why it happens:** Phase 1 only required `["sync"]` for `Mutex`. `TcpListener` and the async runtime require `["rt", "net"]`.
**How to avoid:** Update `Cargo.toml` tokio features to `["rt", "net", "sync"]`.

---

## Code Examples

### GetDeviceInformationResponse XML
```xml
<!-- Source: devicemgmt.wsdl lines 489–519 — literal xs:string elements, no namespace on leaf text -->
<tds:GetDeviceInformationResponse
    xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <tds:Manufacturer>Acme Corp</tds:Manufacturer>
  <tds:Model>Cam-1000</tds:Model>
  <tds:FirmwareVersion>1.0.0</tds:FirmwareVersion>
  <tds:SerialNumber>SN-123456</tds:SerialNumber>
  <tds:HardwareId>HW-REV-A</tds:HardwareId>
</tds:GetDeviceInformationResponse>
```

### GetCapabilitiesResponse XML (minimal, Device + Media)
```xml
<!-- Source: onvif.xsd tt:Capabilities (line 3308), tt:DeviceCapabilities (3387), tt:MediaCapabilities (3485) -->
<tds:GetCapabilitiesResponse
    xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
    xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:Capabilities>
    <tt:Device>
      <tt:XAddr>http://HOST:PORT/onvif/device_service</tt:XAddr>
    </tt:Device>
    <tt:Media>
      <tt:XAddr>http://HOST:PORT/onvif/media_service</tt:XAddr>
      <tt:StreamingCapabilities>
        <tt:RTPMulticast>false</tt:RTPMulticast>
        <tt:RTP_TCP>true</tt:RTP_TCP>
        <tt:RTP_RTSP_TCP>true</tt:RTP_RTSP_TCP>
      </tt:StreamingCapabilities>
    </tt:Media>
  </tds:Capabilities>
</tds:GetCapabilitiesResponse>
```

### GetServicesResponse XML
```xml
<!-- Source: devicemgmt.wsdl tds:Service type (lines 39–70) -->
<tds:GetServicesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <tds:Service>
    <tds:Namespace>http://www.onvif.org/ver10/device/wsdl</tds:Namespace>
    <tds:XAddr>http://HOST:PORT/onvif/device_service</tds:XAddr>
    <tds:Version><tt:Major xmlns:tt="http://www.onvif.org/ver10/schema">2</tt:Major><tt:Minor xmlns:tt="http://www.onvif.org/ver10/schema">42</tt:Minor></tds:Version>
  </tds:Service>
</tds:GetServicesResponse>
```

### GetScopesResponse XML
```xml
<!-- Source: onvif.xsd tt:Scope (line 2386); ScopeDefinition is enum Fixed|Configurable -->
<tds:GetScopesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:Scopes>
    <tt:ScopeDef>Fixed</tt:ScopeDef>
    <tt:ScopeItem>onvif://www.onvif.org/type/video_encoder</tt:ScopeItem>
  </tds:Scopes>
  <tds:Scopes>
    <tt:ScopeDef>Fixed</tt:ScopeDef>
    <tt:ScopeItem>onvif://www.onvif.org/Profile/Streaming</tt:ScopeItem>
  </tds:Scopes>
</tds:GetScopesResponse>
```

### GetHostnameResponse XML
```xml
<!-- Source: onvif.xsd tt:HostnameInformation (line 2821) -->
<tds:GetHostnameResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:HostnameInformation>
    <tt:FromDHCP>false</tt:FromDHCP>
    <tt:Name>onvif-device</tt:Name>
  </tds:HostnameInformation>
</tds:GetHostnameResponse>
```

### GetNetworkInterfacesResponse XML
```xml
<!-- Source: onvif.xsd tt:NetworkInterface (line 2414) extends DeviceEntity (has token attr) -->
<tds:GetNetworkInterfacesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:NetworkInterfaces token="eth0">
    <tt:Enabled>true</tt:Enabled>
    <tt:Info>
      <tt:Name>eth0</tt:Name>
      <tt:HwAddress>00:00:00:00:00:00</tt:HwAddress>
      <tt:MTU>1500</tt:MTU>
    </tt:Info>
  </tds:NetworkInterfaces>
</tds:GetNetworkInterfacesResponse>
```

### ServerBuilder wiring in OnvifServer::run()
```rust
// Source: soap-server/src/server.rs — verified API
let soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
        include_bytes!("../wsdl/devicemgmt.wsdl").to_vec(),
        EmbeddedWsdlLoader,
    )
    .path("/onvif/device_service")
    .default_handler(device_handler)      // absorbs all 60+ ops; dispatches internally for 7
    .auth(move |user| {
        if Some(user) == username.as_deref() { password.clone() } else { None }
    })
    .auth_bypass(auth_bypass)
    .build()
    .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

let router = soap_svc.into_router();
let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
axum::serve(listener, router).await?;
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| yaserde derive for ONVIF XML | hand-written quick-xml string format! | Phase 1 (Rust 1.85.1 icu_* constraint) | No macro codegen overhead; explicit XML structure; must declare namespaces manually |
| `Result<(), OnvifError>` trait stubs | Typed return structs per operation | Phase 2 | Enables handler to build response XML from structured data; not serialization library dependency |
| `build()` returns skeletal `OnvifServer` | `run()` actually binds port | Phase 2 (deferred from Phase 1) | run() is the new entry point; `build()` remains for validation only |

---

## WSDL Structure Notes (critical for soap-server integration)

The devicemgmt.wsdl (200KB, 4791 lines) has:
- **No `wsdl:service` element** — soap-server falls to single-service mode, uses `.path()` mount path
- **~60 WSDL operations** in the binding — all must be handled or a `default_handler` must be set
- **Single binding:** `DeviceBinding` bound to portType `Device`
- **SOAP 1.2** (`soap:` prefix maps to `http://schemas.xmlsoap.org/wsdl/soap12/`)
- **tds namespace:** `http://www.onvif.org/ver10/device/wsdl`
- **tt namespace:** `http://www.onvif.org/ver10/schema` (imported types from onvif.xsd)
- **EmbeddedWsdlLoader** must be passed to `from_wsdl_bytes_with_loader` (not `from_wsdl_bytes`) because the WSDL imports onvif.xsd and common.xsd via relative paths

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + tokio `#[tokio::test]` |
| Config file | none — `cargo test` discovers all `#[test]` and `#[tokio::test]` |
| Quick run command | `cargo test --package onvif-server` |
| Full suite command | `cargo test --package onvif-server -- --include-ignored` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DEV-01 | GetSystemDateAndTime returns 200 with UTC time, no auth | integration | `cargo test device_get_system_date_and_time` | ❌ Wave 0 |
| DEV-01 | GetSystemDateAndTime accessible without Security header | integration | `cargo test device_date_time_no_auth_bypass` | ❌ Wave 0 |
| DEV-02 | GetCapabilities returns Device XAddr matching bound address | integration | `cargo test device_get_capabilities_xaddr` | ❌ Wave 0 |
| DEV-03 | GetServices returns Namespace + XAddr per service | integration | `cargo test device_get_services` | ❌ Wave 0 |
| DEV-04 | GetDeviceInformation returns configured DeviceInfo fields | integration | `cargo test device_get_device_information` | ❌ Wave 0 |
| DEV-05 | GetScopes returns standard onvif:// scope URIs | integration | `cargo test device_get_scopes` | ❌ Wave 0 |
| DEV-06 | GetHostname returns HostnameInformation | integration | `cargo test device_get_hostname` | ❌ Wave 0 |
| DEV-07 | GetNetworkInterfaces returns at least one NetworkInterface | integration | `cargo test device_get_network_interfaces` | ❌ Wave 0 |
| AUTH | Valid WS-Security credential → 200 on authenticated op | integration | `cargo test device_auth_valid_credential` | ❌ Wave 0 |
| AUTH | Invalid credential → SOAP auth fault | integration | `cargo test device_auth_invalid_credential` | ❌ Wave 0 |

### Test Infrastructure Strategy

Phase 2 tests should use `axum-test` (already in soap-server dev-dependencies) or spin up a real listener on a random port with `TcpListener::bind("127.0.0.1:0")`. The `axum-test` crate is NOT in onvif-server's dev-dependencies yet.

**Recommended:** Use `axum-test` for in-process testing — no port conflicts, faster than OS TCP. Add to dev-dependencies:
```toml
axum-test = "20"
```

Alternatively, use `tokio::net::TcpListener::bind("127.0.0.1:0")`, get the assigned port, spin the server as a background task, send HTTP with `reqwest` or `hyper` — but this requires `reqwest` in dev-deps.

**Simpler option (no new deps):** Write tests that call `DeviceServiceHandler::handle()` directly with fabricated Bytes, skipping the HTTP layer. This tests dispatch and XML generation without needing a live server.

### Sampling Rate
- **Per task commit:** `cargo test --package onvif-server`
- **Per wave merge:** `cargo test --package onvif-server -- --include-ignored`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/device_management.rs` — integration tests for DEV-01 through DEV-07 + auth
- [ ] `axum-test = "20"` in `[dev-dependencies]` (if HTTP-level tests chosen over unit-level)
- [ ] `src/service/mod.rs` — new module file if `src/service/` directory is used

---

## Open Questions

1. **Advertised host in XAddrs**
   - What we know: XAddr must reflect the server's bound address; hardcoding `localhost` breaks real clients
   - What's unclear: Should `OnvifServerBuilder` expose an `advertised_host()` method, or derive the host from the bound interface?
   - Recommendation: Add `pub advertised_host: Option<String>` to `OnvifServerBuilder`; default to `"0.0.0.0"` in constructed XAddrs; document that consumers set this to the camera's LAN IP

2. **GetNetworkInterfaces: real detection vs. configured stub**
   - What we know: `tt:NetworkInterface` needs Enabled, Info (Name, HwAddress, MTU), optional IPv4; many consumers won't care about this field
   - What's unclear: Should the default trait implementation call `if-addrs` to auto-detect, or return a static stub?
   - Recommendation: Default implementation returns a single stub interface with placeholder MAC; add `if-addrs` as an optional dep in Phase 5 if real discovery is needed

3. **`run()` return type and shutdown signal**
   - What we know: `axum::serve()` runs forever unless the TcpListener is dropped or a shutdown signal is sent
   - What's unclear: Should `run()` accept a shutdown signal (`CancellationToken`, `oneshot::Receiver<()>`) in this phase?
   - Recommendation: Phase 2 scope is `run()` that runs until error. Graceful shutdown is a Phase 5+ concern.

---

## Sources

### Primary (HIGH confidence)
- `soap-server/src/server.rs` (read directly) — `ServerBuilder` API: `.from_wsdl_bytes_with_loader()`, `.path()`, `.handler()`, `.default_handler()`, `.auth()`, `.auth_bypass()`, `.build()` → `SoapService`; `SoapService::into_router()` → `axum::Router`
- `soap-server/src/handler.rs` (read directly) — `SoapHandler` trait: `async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault>`; `FnHandler` wrapper
- `soap-server/src/dispatch.rs` (read directly) — `build_dispatch_table` semantics; `default_handler` absorbs unregistered ops; `UnregisteredOperation` error behavior
- `soap-server/src/lib.rs` (read directly) — public API exports
- `wsdl/devicemgmt.wsdl` (read directly) — operation list; no wsdl:service element; tds namespace; ~60 operations in binding
- `wsdl/onvif.xsd` (read directly) — `tt:SystemDateTime`, `tt:Scope`, `tt:HostnameInformation`, `tt:NetworkInterface`, `tt:Capabilities`, `tt:DeviceCapabilities`, `tt:MediaCapabilities` structures
- `src/server.rs`, `src/traits/device.rs`, `src/generated/types.rs`, `src/error.rs`, `src/wsdl_loader.rs` (read directly) — existing Phase 1 code

### Secondary (MEDIUM confidence)
- axum 0.8 `serve()` API — inferred from soap-server Cargo.toml declaring `axum = "0.8"` and standard axum 0.8 patterns; `axum::serve(TcpListener, Router)` is the documented entry point for axum 0.8+

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps read directly from Cargo.toml; no inference
- Architecture: HIGH — soap-server API verified from source; WSDL structure verified from file
- Pitfalls: HIGH — derived directly from soap-server source behavior (UnregisteredOperation, auth_bypass wiring, namespace requirements)
- XML format: HIGH — derived from WSDL/XSD source definitions in onvif.xsd and devicemgmt.wsdl

**Research date:** 2026-04-05
**Valid until:** 2026-05-05 (stable stack — soap-server is a path dep, WSDL is bundled)
