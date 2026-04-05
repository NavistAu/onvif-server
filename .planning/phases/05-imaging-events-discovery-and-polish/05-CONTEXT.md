# Phase 5: Imaging, Events, Discovery, and Polish - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Complete the v1 service surface: GetImagingSettings (trait delegation), Events service (CreatePullPointSubscription, PullMessages, Unsubscribe with in-memory subscription state), WS-Discovery (feature-gated UDP multicast ProbeMatch), and ODM smoke test. This rounds out the crate for general-purpose use beyond Frigate.

</domain>

<decisions>
## Implementation Decisions

### Approach
- Porting operation following best prior art (onvif-rs, python-onvif-zeep, ONVIF spec) — same as Phases 1-4
- Follow established patterns from the research

### Carried Forward from Prior Phases
- Hand-written types (not codegen) — same approach throughout
- ServiceHandler dispatch pattern (extract_local_name + match) proven in Phases 2-4
- Builder already accepts `.imaging_service(impl)` and `.event_service(impl)` — needs wiring in run()
- Router::merge() pattern for multi-service wiring proven
- Token constants all defined

### Claude's Discretion
- ImagingServiceHandler implementation (minimal — single GetImagingSettings operation delegates to trait)
- EventServiceHandler with in-memory subscription state (HashMap<subscription_id, subscription_info>)
- PullMessages queue implementation (Vec<EventNotification> per subscription, consumer pushes events)
- WS-Discovery UDP multicast implementation behind `discovery` feature flag using socket2
- ODM smoke test structure (TEST-03) — what specific operations to test
- Whether Events service needs its own WSDL or shares with Device service
- All technical decisions follow research and DESIGN.md

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- DeviceServiceHandler, MediaServiceHandler, PTZServiceHandler: proven dispatch patterns
- OnvifServer::run() with triple Router::merge() — add Imaging + Events as 4th and 5th services
- ImagingService + EventService trait stubs in src/traits/ — need typed signatures
- EmbeddedWsdlLoader: already serves imaging.wsdl and events.wsdl
- virtual_ptz example: needs extension to implement Imaging + Events traits
- Builder accepts all five service types already

### Established Patterns
- One handler struct per service implementing SoapHandler
- extract_local_name + match for operation dispatch
- XML responses as format strings with inline namespace declarations
- Integration tests calling handler.handle() directly
- TDD: scaffold stubs → implement → enable tests

### Integration Points
- run() needs 4th and 5th ServerBuilder blocks for Imaging and Events
- virtual_ptz example needs ImagingService + EventService implementations
- WS-Discovery is a separate tokio task (not part of the SOAP router) — spawned in run() when feature enabled
- ODM smoke test (TEST-03) exercises the full multi-service surface

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond what research and ONVIF spec define. WS-Discovery is feature-gated per DESIGN.md — not needed for Frigate (direct URL config) but useful for NVR auto-discovery.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 05-imaging-events-discovery-and-polish*
*Context gathered: 2026-04-05*
