# Phase 2: Device Management - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Working ONVIF device endpoint with auth, GetSystemDateAndTime (auth-exempt), GetCapabilities, GetServices, GetDeviceInformation, GetScopes, GetHostname, and GetNetworkInterfaces. This is the first service phase — validates end-to-end wiring from HTTP request through soap-server dispatch to trait method invocation and XML response.

</domain>

<decisions>
## Implementation Decisions

### Approach
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `OnvifServer`/`OnvifServerBuilder` in `src/server.rs`: stores services as `Arc<dyn XService>`, auth credentials, auth_bypass set, port
- `OnvifError` in `src/error.rs`: 3 variants with `into_soap_fault()` producing `xmlns:ter` faults
- `DeviceService` trait in `src/traits/device.rs`: all 7 Device Management operations with `not_implemented()` defaults
- `EmbeddedWsdlLoader` in `src/wsdl_loader.rs`: serves all ONVIF WSDLs via `include_bytes!`
- `DeviceInfo` stub in `src/generated/types.rs`: 5-field struct (manufacturer, model, firmware_version, serial_number, hardware_id)
- Token constants in `src/constants.rs`: PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV
- soap-server: `ServerBuilder`, `SoapService`, `SoapHandler`, `FnHandler`, `SoapFault`, `FaultCode`, WS-Security

### Established Patterns
- Builder pattern with `OnvifServer::builder().device_service(impl).auth("user", "pass").port(8080).build()`
- `Arc<dyn XService>` for dynamic dispatch of trait implementations
- `async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault>` is the soap-server handler contract
- Auth bypass via `HashSet<String>` of operation names — `GetSystemDateAndTime` already pre-registered
- Hand-written types (not generated) — yaserde/xsd-parser codegen deferred due to Rust 1.85.1 constraint

### Integration Points
- `OnvifServer::build()` needs to create `SoapService` from devicemgmt.wsdl bytes + dispatch handler
- `OnvifServer::run()` needs to call soap-server's `ServerBuilder` to bind port and start axum listener
- soap-server `auth_bypass` needs to receive the builder's bypass set
- soap-server `auth()` needs credentials from the builder

</code_context>

<specifics>
## Specific Ideas

No specific requirements — porting operation following best prior art. Research findings, DESIGN.md, and ONVIF spec are the authoritative references.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 02-device-management*
*Context gathered: 2026-04-05*
