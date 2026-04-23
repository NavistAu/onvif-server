# onvif-server — Round 1 Review (Codex)
Date: 2026-04-21
Reviewer: OpenAI Codex (gpt-5.3-codex, v0.122.0)
Codex session: 019db881-7788-7853-9b46-e57860f37ca1
Review base: commit fccda57 (initial commit) → HEAD (b8c0083)

## Blockers (must fix before 0.1.0 publish)

- [BLOCK-OS-CDX-01] PTZ coordinate parsing silently coerces malformed values to 0.0 — triggers unintended movement
  File: src/service/ptz.rs:299-300
  Codex finding (P1 — highest priority): PTZ coordinate parsing uses `.parse::<f32>().unwrap_or(0.0)`. If a client sends malformed attribute values (e.g. `x="abc"`, `y="NaN"`), the server silently coerces them to zero and forwards the invalid coordinates to movement commands. This can trigger unintended PTZ movement — a real physical camera would move unexpectedly. Additionally, the client error is hidden (the server accepts the malformed request instead of returning an ONVIF fault).
  Impact: Correctness + potential physical-world safety issue. A physical PTZ camera responding to garbage input with real movement is unacceptable. Release-blocker.
  Fix: Replace `.unwrap_or(0.0)` with `map_err(|_| OnvifError::InvalidArgument("malformed coordinate".into()))?` and propagate as ONVIF fault.

- [BLOCK-OS-CDX-02] `OnvifServerBuilder::build()` never returns `BuildError::MissingRequiredService` — validation deferred to runtime
  File: src/server.rs:331-333
  Codex finding (P2): `build()` always returns `Ok(OnvifServer {...})` even when required service handlers (device_service, media_service, ptz_service, imaging_service, event_service) are all `None`. The actual validation happens in `run()` via `.ok_or("device_service is required...")`. This defeats the purpose of the builder pattern — build-time errors should be caught at build time, not at runtime. The `BuildError::MissingRequiredService` variant exists but is never used.
  Impact: API contract violation — consumers calling `build()?.run()` get runtime panics from mis-positioned error surfacing. Release-blocker.
  Fix: In `build()`, check `self.device_service.is_none()` (and other required services) and return `Err(BuildError::MissingRequiredService("device_service".into()))`.

- [BLOCK-OS-CDX-03] docs.yml has duplicate `on.push` YAML key — main branch push does NOT trigger docs workflow
  File: .github/workflows/docs.yml:4-7
  Codex finding (P2): The workflow defines `on.push` twice. In YAML, duplicate keys cause the second definition to overwrite the first. The `branches: [main]` trigger is overwritten by `tags: ['v*']`. In practice, docs/book deployment does NOT run on main branch pushes — only on tag pushes. This means documentation lags behind main continuously.
  Impact: CI correctness — docs workflow behaves differently from stated intent.
  Fix: Merge both push triggers into one `push:` key:
  ```yaml
  on:
    push:
      branches: [main]
      tags: ['v*']
    workflow_dispatch:
  ```

- [BLOCK-OS-CDX-04] ImagingSettings response emits two separate `<tt:WhiteBalance>` elements instead of one
  File: src/service/imaging.rs:109-117
  Codex finding (P2): When both `white_balance_cr_gain` and `white_balance_cb_gain` are populated, the response serializes two separate `<tt:WhiteBalance>` elements (each with one child gain element). ONVIF clients expect one `<tt:WhiteBalance>` element with both gain fields as children. This malformed response fails schema validation in strict clients (e.g. python-onvif-zeep, Synology NAS camera scanners).
  Impact: Interoperability blocker — clients receiving malformed ImagingSettings responses will reject the device or crash.
  Fix: Emit a single `<tt:WhiteBalance>` element containing both `<tt:CrGain>` and `<tt:CbGain>` as children.

## Non-blockers (should fix / document known limitations)

- [NB-OS-CDX-01] build.rs is a stub — causes consumers to expect codegen that doesn't happen
  File: build.rs
  Codex noted build.rs declares `rerun-if-changed=wsdl/` but does nothing with the wsdl/ directory. This creates a misleading signal for crate consumers.
  Recommendation: Remove build.rs or document its future intent. See docs/roadmap.md which explicitly flags this.

- [NB-OS-CDX-02] `events.wsdl` references external OASIS/W3C URLs — import resolution may fail in offline builds
  File: wsdl/events.wsdl:13-19
  Codex noted events.wsdl imports from `http://docs.oasis-open.org/wsn/bw-2.wsdl`, `http://docs.oasis-open.org/wsrf/rw-2.wsdl`, `http://www.w3.org/2005/08/addressing/ws-addr.xsd`, and others. The `EmbeddedWsdlLoader` handles these by name matching, but if the loader misses any URL the import is silently skipped. In offline or air-gapped environments this could cause startup failures.
  Recommendation: Document the `EmbeddedWsdlLoader` coverage and list which external imports are handled vs silently skipped.

## Codex raw output notes

- Codex reviewed the full diff from initial commit (fccda57) to HEAD (b8c0083)
- Review transcript: /tmp/codex-onvif-run.txt (27213 lines, includes all tool calls)
- Codex ran in read-only sandbox mode; no file modifications made
- Priority labels: P1 = critical/safety, P2 = significant issue, P3 = moderate
- Codex found the ImagingSettings WhiteBalance structure bug and PTZ coordinate coercion bug that Claude missed

## Summary
4 blockers, 2 non-blockers.

Codex P1 findings (critical): PTZ coordinate silent coercion (CDX-01).
Codex P2 findings (blockers): build() validation gap (CDX-02), docs.yml YAML key override (CDX-03), WhiteBalance response malformation (CDX-04).
