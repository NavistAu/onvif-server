# Phase 4: PTZ Service - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Full PTZ control surface: node/configuration discovery (GetNodes, GetNode, GetConfigurations, GetConfiguration, GetConfigurationOptions, GetServiceCapabilities), movement operations (RelativeMove, ContinuousMove, AbsoluteMove, Stop, GetStatus), and preset operations (GetPresets, GotoPreset, SetPreset, RemovePreset). Plus Frigate autotracker end-to-end compat test and virtual_ptz example. This is the core deliverable — all Frigate-specific pitfalls live here.

</domain>

<decisions>
## Implementation Decisions

### Approach
- Porting operation following best prior art (onvif-rs, python-onvif-zeep, ONVIF spec, Frigate source) — same as Phases 1-3
- Follow established patterns from the research — PTZServiceHandler mirrors DeviceServiceHandler/MediaServiceHandler dispatch pattern

### Carried Forward from Prior Phases
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `DeviceServiceHandler` + `MediaServiceHandler`: proven dispatch pattern to replicate
- `OnvifServer::run()` with Router::merge() for multi-service wiring
- Token constants: PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV, PROFILE_TOKEN
- `OnvifError` + `not_implemented()` for unimplemented operations
- `PTZService` trait stub in `src/traits/ptz.rs`: needs typed signatures
- `EmbeddedWsdlLoader`: already serves ptz.wsdl

### Established Patterns
- One handler struct per service implementing `SoapHandler`
- `extract_local_name` + match for operation dispatch
- XML responses as format strings with inline namespace declarations
- Integration tests calling handler.handle() directly with SOAP XML
- TDD: scaffold stubs → implement → enable tests

### Integration Points
- `run()` needs third `ServerBuilder` block for PTZ service at `/onvif/ptz_service`
- PTZ tokens must match those used in Media GetProfiles response (PTZ_CONFIG_TOKEN, PTZ_NODE_TOKEN)
- Frigate compat test exercises cross-service flow: GetProfiles (Media) → PTZ operations
- virtual_ptz example demonstrates the full consumer API surface

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond what the research and ONVIF spec define. The critical constraints are all Frigate compatibility pitfalls documented in the project research:
- TranslationSpaceFov URI must be byte-for-byte exact (pitfall #3)
- MoveStatus="true" in GetServiceCapabilities (pitfall #4)
- Token consistency across services (pitfall #5)
- Profile PTZConfiguration structure validated by Frigate (pitfall #2, addressed in Phase 3)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 04-ptz-service*
*Context gathered: 2026-04-05*
