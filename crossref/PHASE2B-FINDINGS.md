# crossref Phase 2b — conformance findings

Real conformance findings surfaced by the Layer-2 oracle / Docker pipeline. These are
NOT forced green; affected scenarios will report `SutFail` in the Task 6 run and are not
promoted to `verified` until resolved (or explicitly accepted as a recorded
`KnownDivergence`).

## Resolved

- **PTZ `Stop` unreachable over SOAP.** `wsdl/ptz.wsdl` named the Stop request element
  `StopRequest` (vs the ONVIF-standard `Stop`), so the dispatch table registered
  `{ns}StopRequest` and `<tptz:Stop>` requests returned a Sender fault "Action not
  supported". **Fixed** in commit `ba5480a` (element renamed `StopRequest` → `Stop`); the
  `ptz_stop` scenario flipped fault→success and its baseline regenerated.

## Open

### F-1: `GetStatus` response omits required `PTZStatus/UtcTime` (schema-invalid)

- **Discovered:** Task 3, via the ONVIF schema oracle (offline `xmllint` against
  `ptz-body.xsd` + `common.xsd`).
- **Severity:** real onvif-server conformance bug — response is **schema-invalid**.
- **Detail:** `tt:PTZStatus` (defined in `common.xsd`, `xs:complexType name="PTZStatus"`)
  requires a `tt:UtcTime` (`xs:dateTime`) child element (no `minOccurs` → `minOccurs="1"`).
  Our handler `src/service/ptz.rs::handle_get_status` (~line 281) emits only
  `<tptz:PTZStatus><tt:MoveStatus>…</tt:MoveStatus></tptz:PTZStatus>` — no `UtcTime`.
  xmllint verdict: `Element '{…schema}PTZStatus': Missing child element(s). Expected is
  one of ( {…schema}Error, {…schema}UtcTime ).`
- **Why not auto-fixed:** only the PTZ `Stop` product fix was authorized this session.
  `UtcTime` is a non-deterministic timestamp, so a fix also requires the controlled
  fixture to emit a fixed time and the crossref harness to add a `UtcTime` mask for
  `ptz_get_status` (timestamps are masked). That is a product + harness design decision
  for the operator.
- **Effect on Task 6:** `ptz_get_status` (scenario `reference_mode = "none"`) will be a
  `SutFail` (oracle schema-invalid) and will NOT be promoted to `verified`.
- **Suggested fix (when authorized):** emit `<tt:UtcTime>` in `handle_get_status` (fixture
  supplies a deterministic value), add a `ptz_get_status` UtcTime mask, regenerate the
  baseline. Check other operations whose ONVIF type requires a timestamp/`UtcTime`
  (e.g. `GetSystemDateAndTime`, events `CurrentTime`) for the same class of issue — Task 6
  will surface any others comprehensively.
