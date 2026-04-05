# Phase 3: Media Service - Context

**Gathered:** 2026-04-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Profile S media metadata — GetProfiles (with correct PTZConfiguration structure for Frigate), GetStreamUri, GetVideoSources, GetVideoSourceConfigurations, GetVideoEncoderConfigurations, GetSnapshotUri. No actual video streaming — consumers provide RTSP/snapshot URLs via trait methods. This phase establishes the profile tokens that thread through all subsequent PTZ operations.

</domain>

<decisions>
## Implementation Decisions

### Approach
- Porting operation following best prior art (onvif-rs, python-onvif-zeep, ONVIF spec) — same as Phases 1-2
- Follow established patterns from the research — don't reinvent where prior art already defines the right answer

### Carried Forward from Prior Phases
- Hand-written types (not codegen) — same approach as Phase 1-2
- DeviceServiceHandler dispatch pattern (extract_local_name + match) — reuse for MediaServiceHandler
- Token constants already defined: PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_NODE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV
- Builder already accepts `.media_service(impl)` — just needs wiring in `run()` like Device was wired in Phase 2

### Claude's Discretion
- MediaServiceHandler implementation following the DeviceServiceHandler pattern from Phase 2
- Profile XML structure: Must include PTZConfiguration with DefaultContinuousPanTiltVelocitySpace set to TRANSLATION_SPACE_FOV URI — critical Frigate pitfall #2 from research
- MediaService trait signatures: typed returns for GetProfiles, GetStreamUri, GetVideoSources, etc.
- Type expansion for Media-specific structures (Profile, VideoSource, VideoSourceConfiguration, VideoEncoderConfiguration)
- Multi-service router merging in run() — adding Media service alongside Device service
- All technical decisions follow research findings, DESIGN.md, and the Phase 2 DeviceServiceHandler as pattern reference

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `DeviceServiceHandler` in `src/service/device.rs`: proven dispatch pattern (extract_local_name + match → XML response)
- `OnvifServer::run()` in `src/server.rs`: already wires Device service via ServerBuilder — Media follows same pattern
- Token constants in `src/constants.rs`: PROFILE_TOKEN, VIDEO_SOURCE_TOKEN, PTZ_CONFIG_TOKEN, TRANSLATION_SPACE_FOV
- `OnvifError` + `not_implemented()` default pattern for unimplemented operations
- `MediaService` trait stub in `src/traits/media.rs`: needs typed return signatures like Device was updated in Phase 2
- `EmbeddedWsdlLoader`: already serves media.wsdl

### Established Patterns
- One `XServiceHandler` implementing `SoapHandler` per ONVIF service
- `default_handler` registration on soap-server's `ServerBuilder` per service
- XML responses built as format strings with inline namespace declarations (tds:/tt: etc.)
- Integration tests calling handler.handle() directly with fabricated SOAP XML bytes

### Integration Points
- `run()` must create `MediaServiceHandler` and register it as a second `SoapService` merged via Router::merge()
- Profile tokens from constants.rs must be used in GetProfiles response — these same tokens will be referenced by PTZ in Phase 4
- GetProfiles PTZConfiguration must reference PTZ_CONFIG_TOKEN and TRANSLATION_SPACE_FOV — Frigate checks this

</code_context>

<specifics>
## Specific Ideas

No specific requirements — porting operation following best prior art. The critical constraint is Frigate compatibility: GetProfiles must include PTZConfiguration with DefaultContinuousPanTiltVelocitySpace set to the TranslationSpaceFov URI, or Frigate silently disables PTZ autotracking.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-media-service*
*Context gathered: 2026-04-05*
