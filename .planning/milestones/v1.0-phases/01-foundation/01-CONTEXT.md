# Phase 1: Foundation - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Crate scaffold, error types, WSDL loader, ONVIF type definitions, token constants, and builder skeleton. Everything downstream service phases depend on. No ONVIF service operations are implemented in this phase.

</domain>

<decisions>
## Implementation Decisions

### Approach
- This is a porting operation informed by the best prior art (onvif-rs, python-onvif-zeep, ONVIF spec), not a greenfield design
- Follow established patterns from the research — don't reinvent where prior art already defines the right answer

### Claude's Discretion
- Type definition strategy: Try Option A (onvif-rs schema crates) first; fall back to Option B (xsd-parser build.rs) if yaserde 0.7 vs 0.12 conflict is irreconcilable. Research recommends a 30-minute compile spike to determine viability.
- WSDL/XSD bundling: Bundle all service WSDLs upfront (devicemgmt, media, ptz, imaging, events) plus shared XSDs — they're small static files and downstream phases need them. Use `include_bytes!` for compile-time embedding.
- Builder API surface: Skeleton that compiles and accepts service registration calls. Functional wiring happens in Phase 2 when the first service (Device Management) is implemented.
- Token constants: Define all crate-level `pub const` tokens (profile, video source, PTZ node, PTZ config) from day one per research recommendation. These are defaults; consumer overridability is a Phase 2+ concern if needed.
- Error types: `OnvifError` with variants matching research (NotImplemented, InvalidArgument, ActionNotSupported), mapping to SOAP faults with `xmlns:ter` namespace per pitfall #7.
- All technical decisions (architecture, patterns, naming, module structure) follow the research findings and DESIGN.md as starting points.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `soap-server` sibling crate at `~/ws/soap-server`: provides `SoapHandler` trait (bytes in/out), `ServerBuilder`, `SoapService`, `FileWsdlLoader`, `WsdlLoader` trait, `SoapFault`, WS-Security
- `soap-server` handler interface: `async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault>`
- `FnHandler` wrapper for closure-based handlers

### Established Patterns
- soap-server uses `axum 0.8`, `tokio 1`, `async-trait`, `bytes` — onvif-server must match versions exactly
- `WsdlLoader` trait for serving WSDL bytes — `EmbeddedWsdlLoader` will implement this with `include_bytes!`
- `SoapFault` with `FaultCode::Sender` pattern for error responses

### Integration Points
- Path dependency on `soap-server` at `~/ws/soap-server`
- `OnvifServer` builder will create `SoapService` instances per ONVIF service and merge via `Router::merge()`
- Auth bypass registration for `GetSystemDateAndTime` (Phase 2, but builder skeleton should anticipate it)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — this is a port following best prior art. Research findings and DESIGN.md are the authoritative references for all implementation choices.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-04-05*
