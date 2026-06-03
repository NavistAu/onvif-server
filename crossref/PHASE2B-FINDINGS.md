# crossref Phase 2b — conformance findings

Real conformance findings surfaced by the Layer-2 oracle / Docker pipeline. Findings are
NOT forced green; a scenario stays `SutFail` (red) until its finding is resolved.

**Context:** Phase 2a (Layer-1) only byte-diffed responses against self-captured
baselines — it never schema-validated. The Layer-2 ONVIF schema oracle (Xerces, validating
against the real `onvif.xsd`/`common.xsd`) is the first time onvif-server responses have been
checked against the ONVIF XSDs. It revealed **six pre-existing product conformance bugs**
(schema-invalid responses) plus one harness/bundle limitation.

**Status: ALL findings resolved — the suite is release-green** (`--release-green` exits 0:
29 scenarios all `verified`, 0 unverified, empty expected-failures baseline).
- F-1,F-2,F-4,F-5,F-6 (onvif-server) — `6006bef`
- F-3 (soap-server SOAP 1.2 `env:Detail`) — `13b907e`
- F-7 (WS-Discovery device endpoint UUID was random per probe; now stable + pinned/asserted) — `c857ee2`
- A-1 (ws-addr.xsd EPR `ReferenceParameters`) — `5695c33`

Beyond the findings, a release-review hardening pass (`1a6772f`): Layer-2 now verifies
discovery (was skipped), canonical evidence is masked-then-C14N (deterministic), multistep
scenarios promote per step, `Step.inject` is enforced, and `--release-green` gates on
zero-red + zero-unverified + empty-baseline. The previously-vacuous `scopes_match` /
`stable_endpoint_uuid` invariants now assert real values.

## Resolved

- **PTZ `Stop` unreachable over SOAP** — `wsdl/ptz.wsdl` named the Stop request element
  `StopRequest` (vs ONVIF-standard `Stop`). **Fixed** `ba5480a` (this session, authorized).

## Resolved — real product conformance bugs (schema-invalid responses)

All confirmed by the live oracle (Xerces) in the first Task 6 run, then **fixed and verified
passing** in the re-run (onvif-server `6006bef`; soap-server F-3 `13b907e`). Each scenario is
now schema-valid and promoted to `verified`. Descriptions retained for the record.

### F-1: `GetStatus` — `PTZStatus` missing required `UtcTime`
- Scenario `ptz_get_status`. Oracle: `cvc-complex-type.2.4.b: content of 'tptz:PTZStatus'
  not complete. One of {tt:Error, tt:UtcTime} expected.`
- `tt:PTZStatus` (common.xsd) requires `UtcTime` (`xs:dateTime`, minOccurs=1). Handler
  `src/service/ptz.rs::handle_get_status` (~L281) emits only `MoveStatus`.
- Fix needs: emit `tt:UtcTime` (fixture supplies a deterministic value) + add a `UtcTime`
  mask for `ptz_get_status` + regen baseline.

### F-2: `GetImagingSettings` — `WhiteBalance` missing required `Mode`
- Scenario `imaging_get_imaging_settings`. Oracle: `Invalid content starting with 'CrGain'.
  One of {tt:Mode} expected.`
- `tt:WhiteBalance20` sequence requires `Mode` before `CrGain`/`CbGain`. The imaging handler
  emits `WhiteBalance` without `Mode` (or wrong order).

### F-3: malformed-coord fault — `env:Detail` has character children
- Scenario `ptz_absolute_move_malformed_coord`. Oracle (soap12-envelope, authoritative):
  `cvc-complex-type.2.3: Element 'env:Detail' cannot have character [children], type's
  content is element-only.`
- SOAP 1.2 `env:Detail` is element-only; the fault renderer puts text directly in `Detail`.
  Likely in `soap-server`'s `SoapFault` detail rendering. Affects ALL faults that carry a
  text detail (not just PTZ) — worth a broad check.

### F-4: `GetConfigurations` — `PTZConfiguration` vs schema's `PTZConfigurations`
- Scenario `ptz_get_configurations`. Oracle: `Invalid content starting with
  'tptz:PTZConfiguration'. One of {tptz:PTZConfigurations} expected.`
- Element-name mismatch between the handler output and `wsdl/ptz.wsdl`'s
  `GetConfigurationsResponse` (same CLASS of bug as PTZ Stop: WSDL vs handler disagreement).
  Determine which is ONVIF-correct (standard is `PTZConfiguration`, repeatable) and reconcile
  the WSDL or the handler.

### F-5: `GetCapabilities` — `Capabilities` children out of sequence + missing required
- Scenario `device_get_capabilities`. Oracle: `tt:Media` incomplete (StreamingCapabilities
  expected); `Imaging` found where `Extension` expected; `tt:Events` incomplete
  (WSPausableSubscriptionManagerInterfaceSupport expected).
- `tt:Capabilities` is an ordered `xs:sequence` (Analytics, Device, Events, Imaging, Media,
  PTZ, Extension). Our response orders Device, Media, PTZ, Imaging, Events — a sequence-order
  violation — and omits required sub-elements (e.g. `tt:MediaCapabilities/StreamingCapabilities`).

### F-6: `GetEventProperties` — response missing required `tev:` dialect elements
- Scenario `events_get_event_properties`. Oracle: `GetEventPropertiesResponse` not complete;
  one of {tev:MessageContentFilterDialect, tev:ProducerPropertiesFilterDialect,
  tev:MessageContentSchemaLocation} expected.
- These are `events/wsdl`-namespace elements defined in the events embedded schema
  (authoritative — NOT the wsn-b2 stub). The handler omits required response fields.

### F-8: ONVIF `ter:` subcode silently dropped from SOAP 1.2 faults
- Found by a unit test (`ptz_unimplemented_ops_return_action_not_supported`) during 0.1.0
  release prep, not by the oracle. The follow-up F-3 warned about — F-3's fix made SOAP 1.2
  `env:Detail` element-only by emitting ONLY `detail_xml` and dropping the text `detail`
  field; but `OnvifError::into_soap_fault()` put the well-formed `<ter:fault><ter:subcode>…`
  XML in the text `detail` field, so it was silently discarded. Every ONVIF fault
  (NotImplemented/ActionNotSupported/InvalidArgVal) reached SOAP 1.2 clients with NO `ter:`
  code — only `env:Receiver`/`env:Sender` + a reason string.
- Fix: route the (already well-formed) detail XML via `.with_detail_xml(...)` so the SOAP 1.2
  renderer emits it inside `<env:Detail>`. Verified: the ter: subcode now appears on the wire;
  full onvif test suite green; crossref Layer-2 `--release-green` still PASSES 29/29 (the
  `ptz_absolute_move_malformed_coord` fault now carries `<env:Detail>` and still validates,
  since SOAP 1.2 `Detail` is `xs:any` lax).

## Open — harness/bundle limitations (NOT confirmed product bugs)

### A-1: `events_pull_messages` — ws-addressing `ReferenceParameters` rejected
- Oracle: `Invalid content 'wsa:ReferenceParameters'. One of {WC[##other:addressing]}
  expected.` Caused by the harness `##any`→`##other` UPA rewrite applied to the minimal
  `ws-addr.xsd` stub's EPR model, which then rejects an addressing-namespace child. This is a
  schema-bundle limitation (ws-addr is a minimal stub; events-body is non-authoritative per
  SCHEMAS.md), not a confirmed onvif-server bug. Needs a targeted rewrite exclusion or a fuller
  ws-addr.xsd to confirm/deny.

## Reference-server (onvif-srvd) triage — legitimate device differences

Per spec §6 ("avoid false authority"), onvif-srvd is only an authority where both devices can
be pinned to equivalent output. The Task 6 run confirmed onvif-srvd legitimately diverges from
our fixture for most ops, so those scenarios are downgraded `reference_mode = "none"`
(oracle + invariants + our-vs-baseline remain in force):

- `device_get_services` (srvd emits bare-authority XAddrs, no `/onvif/...` path) → `none`.
- `media_get_profiles` (srvd has different profile tokens/structure) → `none`.
- `device_get_capabilities` (srvd capability set differs) → `none` (also independently F-5).
- `device_get_device_information_authed` — **kept** `srvd_exact`: device-info VALUES match
  onvif-srvd exactly; the comparison is fixed to compare the oracle-C14N body child (dropping
  the SOAP Header, where srvd emits an empty `<Header/>` and we omit it — a SOAP-optional,
  non-conformance-relevant difference). This is onvif-srvd's one genuine authority op.
