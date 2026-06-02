# crossref Phase 2 — onvif-server conformance & interop suite (design)

**Date:** 2026-06-02
**Status:** Approved design (v1). Scope: **Phase 2** — applying the `crossref` framework
(built and proven for `soap-server` in Phase 1) to **onvif-server**.
**Working name:** `crossref` (per-repo, in `onvif-server/crossref/`).

---

## 1. Purpose & relationship to Phase 1

Phase 1 built a differential conformance + interop harness for `soap-server` and proved
its value (it caught a real envelope-parsing bug and externally corroborated SOAP
1.1/1.2 + WS-Security conformance against Apache CXF + Python Zeep). Phase 2 applies the
**same framework** to `onvif-server` to anchor ONVIF correctness to independent
authorities rather than our own judgment.

**Key difference from Phase 1:** ONVIF responses are NOT byte-comparable between
independent implementations — a real device returns its own tokens, capabilities,
profiles, timestamps, and message IDs (spec §9 caveat: ONVIF conformance is
"structural/schema-level + masked, not exact content"). So Phase 2 does **not** lean on
an our-vs-reference byte diff as the primary signal. Instead:

- **Primary authority = ONVIF XSD schema validity** (the Java XML oracle / Xerces),
- **plus targeted semantic invariants** in Rust for known ONVIF risks (the round-2 bug
  classes),
- **plus `onvif-srvd` structural comparison only for stable read-only operations** where
  both devices can be pinned to equivalent output,
- **plus real `python-onvif-zeep` interop** as the client-facing gate.

## 2. Placement & packaging

Per the Phase 1 design (§3, "each crate carries its own in-repo harness; no shared
dependency, no separate repo"), Phase 2 **ports the proven Phase-1 crossref Rust
framework into `onvif-server/crossref/`** as a **`publish = false` Cargo workspace
member**. The Rust core transfers nearly as-is — copied into this repo, not shared via a
crate:

- `normalize.rs` (path-scoped text + attribute masking, namespace-prefix canonicalization),
- `snapshot.rs` (golden store + `unverified`/`verified` provenance + `canonical/` evidence),
- `scenario.rs` (declarative scenario model),
- `oracle.rs` (HTTP client for the Java XML oracle: `validate`, `c14n`),
- `layer2/` (compose lifecycle, outcome-aware §5.7 verdict model, promotion, report),
- the Java XML oracle container, `manifest.toml`, `docker-compose.yml`, CI workflow.

`onvif-server/crossref/` must be excluded from the onvif-server publish tarball
(`exclude = ["/crossref"]` in `[package]`), verified via `cargo package --list`.

## 3. Architecture

**Authorities (all containerised; host needs only Docker + cargo):**

1. **Java XML oracle** — reused from Phase 1, extended with the vendored ONVIF schema
   bundle (see §4). Validates SOAP envelope + ONVIF body element; performs exclusive C14N.
2. **`onvif-srvd`** (gSOAP reference device) — Dockerized with a **pinned config matching
   our controlled fixture**, used ONLY for the narrow stable read-only subset (§6).
3. **`python-onvif-zeep`** interop client — Dockerized, drives our server through a
   Profile-S operation sequence with `admin/admin` credentials (§12).
4. **Controlled onvif-server binary** — a `crossref/src/bin/controlled_onvif_server.rs`
   built from the existing `examples/virtual_ptz.rs` pattern, configured with a fully
   **pinned deterministic fixture** (§7).

**Two execution layers (mirrors Phase 1):**

- **Layer 1 (Rust, no Docker):** replay `scenarios/*.toml` against the controlled server
  **through the full SOAP envelope/auth/routing stack** (NOT handler-only), normalize
  (mask + prefix-canon), diff vs frozen `unverified` snapshots. Per-commit CI gate;
  proves *unchanged*, not *correct*.
  **Transport (required):** `OnvifServer` currently builds its merged axum router
  *inside* `run()` (server.rs ~189–249) and only exposes a bound-port `run()`. Phase 2a
  MUST add a `pub fn OnvifServer::into_router(self) -> axum::Router` (extract the existing
  router-build; `run()` then calls `into_router()` and binds) so Layer-1 can drive it
  in-process via `axum_test::TestServer` — exactly as soap-server's `SoapService::into_router()`
  enabled Phase 1. This is an additive, non-breaking product change. Handler-only replay is
  prohibited (it would bypass envelope parsing, WS-Security auth, and operation routing).
- **Layer 2 (Docker):** bring up oracle + onvif-srvd + zeep + our controlled server;
  validate every response against the ONVIF schema bundle; apply semantic invariants;
  run the narrow onvif-srvd structural comparison; run zeep interop; assign §5.7
  verdicts; **promote** `unverified`→`verified` with oracle-canonical evidence.
  Nightly / on-demand.

## 4. Schema-oracle mechanics (offline ONVIF schema bundle)

"Validate against the ONVIF XSDs" is made precise. ONVIF operation **response elements**
(e.g. `GetDeviceInformationResponse`) are defined in each WSDL's embedded
`<wsdl:types><xs:schema>` section, which **imports** the shared `onvif.xsd` / `common.xsd`
(and `ws-addr.xsd`, `soap-envelope.xsd`, etc.). The oracle therefore requires an
**offline schema bundle + resolver — NO network fetches at build or run time:**

- A build step (in the oracle's Docker image, or a vendored generator) **extracts the
  embedded `<xs:schema>` from each service WSDL** in `onvif-server/wsdl/` (devicemgmt,
  media, imaging, ptz, events) into standalone `.xsd` files, and copies the shared
  schemas (`onvif.xsd`, `common.xsd`, `ws-addr.xsd`, `soap-envelope.xsd`, `xmlmime.xsd`,
  `xop-include.xsd`, the `wsn-*`/`wsrf-*` schemas for events) alongside.
- Each is registered under a **schema id**: `device-body`, `media-body`, `imaging-body`,
  `ptz-body`, `events-body`, plus the existing `soap12-envelope`. The shared `onvif.xsd`
  / `common.xsd` / `ws-addr.xsd` are registered as importable dependencies.
- The oracle's existing `LSResourceResolver` is extended to resolve every cross-schema
  import (by namespace URI / relative location) to the **local vendored file** — never the
  network. DOCTYPEs that Xerces rejects are stripped as in Phase 1.
- Validation per scenario: the SOAP envelope against `soap12-envelope`; the **body child
  element** (the operation's `…Response`) against its service `*-body` schema. Faults
  validate against the envelope schema. **Namespace preservation (required):** ONVIF
  responses commonly declare namespaces (`tt:`, `tds:`, `tns1:`, …) on the `Envelope`/
  `Header`/`Body` ancestors, not on the body child. The body-child extractor MUST copy all
  in-scope ancestor namespace declarations onto the extracted child element before
  validation, or schema-valid responses will spuriously fail. (soap-server's
  `envelope::extract_body_first_child` already re-emits ancestor ns decls — verified by its
  `parse_envelope_body_bytes_contain_ancestor_ns_declarations` test — and is reused here.) Header validation (WS-Security / WS-Addressing)
  is reported as `unvalidated` unless a scenario declares header schemas (Phase-1 §5.6
  rule carries over).

Source URLs for every vendored schema are recorded in `crossref/comparators/oracle/SCHEMAS.md`.

## 5. Conformance model (per scenario)

1. **Primary gate — schema validity:** oracle validates the envelope + the operation's
   response body element against the §4 bundle. Invalid → `SutFail`.
2. **Semantic invariants (Rust)** for the known ONVIF risk classes (regression-locking
   the round-2 fixes): e.g. exactly one `<tt:WhiteBalance>` element; a malformed PTZ
   coordinate produces a fault (never silent coercion to 0.0); a pull-point response
   carries the WS-Addressing `SubscriptionId`/`To`/`Action` headers; ProbeMatch carries a
   correct `RelatesTo`. Invariant violation → `SutFail`.
3. **onvif-srvd structural comparison — ONLY the §6 subset.** For those ops: send the
   identical request to onvif-srvd, normalize both (mask + prefix-canon), oracle-C14N,
   structurally compare. Disagreement on a schema-valid pair → `ReferenceDisagreement`
   (triage), never an automatic SUT fail.
4. **Verdict:** reuse the outcome-aware §5.7 model — `Pass` / `SutFail` /
   `ReferenceDisagreement` / `KnownDivergence` / `HarnessError`. Declared `outcome` +
   `expected_status` enforced (the Phase-1c hardening). `is_green()` requires every
   verdict be `Pass` or `KnownDivergence` (fails on SutFail/HarnessError/ReferenceDisagreement).

**Invariants run BEFORE masks.** Semantic invariants (§5.2) and the §5.7 comparison are
evaluated on the **raw, pre-mask response**; masking (§8) is applied only when producing
the normalized snapshot/canonical bytes for diffing. This ordering prevents a mask from
hiding the very field an invariant asserts (e.g. discovery `RelatesTo`).

**KnownDivergence semantics (single rule):** a `KnownDivergence` **counts as green** — but
ONLY via an explicit, reviewed, recorded entry (a documented justification committed to the
repo, e.g. a `known_divergences` table keyed by scenario+path+reason). It is never
auto-assigned to a failing scenario to force green. Absent an approved recorded entry, a
divergence is a `ReferenceDisagreement` (non-green). This matches the Phase-1 model
(is_green accepts KnownDivergence) while requiring human sign-off to reach it.

### 5a. Scenario metadata contract (explicit routing — no name-driven dispatch)

To avoid Phase 1's fragile prefix-based routing (`select_sut` matched scenario names),
every `scenarios/*.toml` declares its behavior **explicitly**; the orchestrator routes on
these fields, never on the scenario name:

```toml
name            = "device_get_device_information_authed"
service         = "device"          # device | media | imaging | ptz | events | discovery
operation       = "GetDeviceInformation"
schema_id       = "device-body"     # which oracle schema validates the body child
http_method     = "POST"            # POST | GET (none for discovery — see §11)
expected_status = 200
outcome         = "success"         # success | fault
auth_mode       = "usernametoken"   # none | usernametoken
reference_mode  = "srvd_exact"      # none | srvd_projection | srvd_exact
invariants      = ["single_white_balance"]   # named structural invariants (may be empty)
masks           = ["wsa_message_id"]          # named path-scoped masks from §8 (may be empty)
request_file    = "device_get_device_information_authed.request.xml"
[fault]                             # present only when outcome = "fault"
code            = "Sender"
subcode         = "ter:NotAuthorized"
```

`reference_mode` selects the §6 comparison (`none` = oracle+invariants only;
`srvd_projection` = §6 projection compare; `srvd_exact` = near-exact masked compare).
`invariants` / `masks` reference named registries in Rust (not inline logic), so adding a
scenario is data, not code. This contract is the single source of truth shared by Layer 1
and Layer 2.

## 6. onvif-srvd scope (narrow — avoid false authority)

`onvif-srvd` is compared against ONLY where both devices can be pinned to equivalent,
stable, read-only output. Two comparison modes (set per scenario via `reference_mode`):

- **`srvd_exact`** — full masked structural equality after prefix-canon + C14N. Used only
  for **GetDeviceInformation** (pin both to identical manufacturer/model/firmware/serial).
- **`srvd_projection`** — compare only a defined PROJECTION of each response, not full
  structural equality (two conformant devices legitimately differ in supported features, so
  full equality would yield false `ReferenceDisagreement`). Used for:
  - **GetCapabilities** — projection = the set of advertised service categories + their
    `XAddr` (path compared, host:port authority masked) + version (Major/Minor) + a defined
    set of required capability booleans. Optional/extra capabilities are ignored.
  - **GetServices** — projection = the set of service `Namespace` values + each service's
    `XAddr` (path; authority masked) + `Version`. Extra services on either side are ignored.
  - **GetProfiles** — `srvd_projection` *only if* both pin identical profile/config tokens;
    otherwise `reference_mode = none` (oracle + invariants only).

The projection for each op is a named, documented extractor in Rust (referenced from the
scenario), so what is being compared is explicit and reviewable.

**Explicitly NOT compared against onvif-srvd** (oracle-validity + invariants only — these
legitimately differ between devices and srvd would be a false authority): PTZ moves
(Absolute/Relative/Continuous/Stop), Events (subscription/pull), Discovery, stream &
snapshot URIs, anything with timestamps, subscriptions, or generated UUIDs.

## 7. Deterministic controlled fixture (defined up front)

The controlled onvif-server binary pins every otherwise-volatile value so masks stay
small and snapshot reviews stay clear:

| Field | Pinned value |
|---|---|
| Manufacturer / Model / Firmware / Serial / HardwareId | `Crossref` / `Controlled-1` / `1.0.0` / `CR-0001` / `CR-HW-1` |
| Profile token | `profile_0` |
| Video source token / config token | `vsrc_0` / `vsconf_0` |
| Video encoder config token | `venc_0` |
| PTZ configuration token / node token | `ptzconf_0` / `ptznode_0` |
| Preset token | `preset_1` (fixed) |
| Stream URI / Snapshot URI | `rtsp://<host>:554/stream0` / `http://<host>/snapshot0` |
| Scopes | `onvif://www.onvif.org/Profile/Streaming`, `…/type/video_encoder`, `…/name/Controlled`, `…/location/lab` |
| Hostname | `controlled-onvif` |
| Network interface | name `eth0`, MAC `00:11:22:33:44:55`, fixed IPv4 |
| PTZ status position | pan `0.0`, tilt `0.0`, zoom `0.0`, MoveStatus `IDLE` |
| XAddrs | `http://<host>/onvif/<service>` |

`<host>` (and `:port`) is the only environment-varying part (in-process Layer-1 vs
container Layer-2), and is the only token-like value that gets masked (§8). Everything
else is deterministic and asserted directly.

## 8. Masking table (path-scoped only — spec §5.3, no value-pattern masks)

| Path (local-name) | Reason |
|---|---|
| `Envelope/Header/MessageID` (`wsa:MessageID`) | per-message UUID (events, discovery) |
| `Envelope/Header/RelatesTo` (`wsa:RelatesTo`) | **NOT masked.** Our request fixtures use a FIXED `MessageID`, so `RelatesTo` is deterministic and is asserted by invariant (`relates_to == request MessageID`). Masking it would defeat that invariant (per the "invariants before masks" rule, and resolving the §11 conflict). |
| `…/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/…` | live clock |
| `…/PullMessagesResponse/CurrentTime` and `…/TerminationTime` | live clock |
| `…/CreatePullPointSubscriptionResponse/…/TerminationTime` | live clock |
| subscription reference / `SubscriptionId` (generated) | per-subscription UUID |
| any generated `urn:uuid:` not pinned by the fixture | volatile |
| host:port authority inside `XAddrs` / stream / snapshot / endpoint URIs | environment-varying (Layer-1 vs Layer-2 host); mask the authority only, keep path/token |

Profile/device/PTZ tokens are NOT masked — the §7 fixture makes them deterministic, so
they are asserted directly (a masked token would hide a real regression).

## 9. Auth / WS-Security (first-class)

ONVIF clients depend on WS-Security UsernameToken. onvif-server builds on soap-server's
auth. The controlled server is configured with credentials **`admin/admin`**, auth
required for protected operations, and `GetSystemDateAndTime` (and discovery)
**auth-bypassed** (ONVIF allows unauthenticated device-time). Scenarios:

- **unauth-allowed:** `GetSystemDateAndTime` with no WS-Security header → success.
- **authenticated success:** a protected op (`GetDeviceInformation`) with a valid
  `admin/admin` UsernameToken digest → success.
- **missing/bad auth fault:** a protected op with no / wrong credentials → fault
  (Sender / NotAuthorized class).
- **interop:** `python-onvif-zeep` authenticates with `admin/admin`.

WS-Security request fixtures use unique nonces + correctly recomputed digests (the
Phase-1c lesson: never share a nonce across fixtures).

## 10. Scenario inventory (broad)

All scenarios are XSD-validated (§5.1); the round-2 regression cases carry semantic
invariants (§5.2); the §6 subset additionally gets onvif-srvd comparison.

- **Device:** `GetSystemDateAndTime` (unauth), `GetCapabilities`*, `GetServices`*,
  `GetDeviceInformation`* (+ authed-success / missing-auth-fault variants), `GetScopes`,
  `GetHostname`, `GetNetworkInterfaces`.
- **Media:** `GetProfiles`*(conditional), `GetStreamUri`, `GetSnapshotUri`,
  `GetVideoSources`, `GetVideoSourceConfigurations`, `GetVideoEncoderConfigurations`.
- **Imaging:** `GetImagingSettings` (+ invariant: exactly one `tt:WhiteBalance`).
- **PTZ:** `GetServiceCapabilities` (**required**; invariant: `Capabilities/@MoveStatus == true`
  — regression-locks the MoveStatus-as-attribute compatibility fix), `GetNodes`,
  `GetConfigurations`, `GetConfigurationOptions`, `GetStatus`, `RelativeMove`, `AbsoluteMove`,
  `ContinuousMove`, `Stop`, **malformed-coordinate fault** (required negative —
  regression-locks the PTZ coordinate-coercion fix). Preset lifecycle
  (`GetPresets`/`GotoPreset`) included if deterministic.
- **Events:** `GetEventProperties`, `CreatePullPointSubscription`, `PullMessages` (assert
  WS-Addressing `SubscriptionId` header), **unknown-subscription fault**.
- **Discovery:** valid `ProbeMatch`, and a **non-Probe → no-response** negative.

(* = also compared against onvif-srvd per §6.)

## 11. Discovery authority model

WS-Discovery is not normal ONVIF-service SOAP, and **no WS-Discovery XSD is vendored** in
the repo. `ws-addr.xsd` alone CANNOT validate a `ProbeMatch` (the ProbeMatch body lives in
the WS-Discovery namespace, not WS-Addressing). Discovery is therefore **strictly
structural-only with invariants** — no oracle schema validation and no onvif-srvd diff.
The Probe request fixture uses a **fixed `MessageID`** so `RelatesTo` is deterministic.
Invariants asserted on the raw response:

- correct `wsa:RelatesTo` == the request Probe's (fixed) `MessageID`;
- a **stable endpoint UUID** (pinned by the fixture — NOT masked, so a regression in
  endpoint identity is caught);
- `Types` present (e.g. `tds:Device`), `Scopes` present and matching the fixture;
- `XAddrs` properly escaped and pointing at our service;
- **no response** to a non-Probe payload.

(Vendoring the WS-Discovery + WS-Addressing schemas to enable oracle validation of
ProbeMatch is an optional future enhancement, explicitly out of scope for Phase 2.)

## 12. Interop (python-onvif-zeep, Profile-S flow)

A `zeep-onvif-client` container runs a real Profile-S sequence against our controlled
server using `admin/admin`: connect + pull WSDLs, `GetDeviceInformation`,
`GetCapabilities`, `GetProfiles`, `GetStreamUri`, PTZ `GetStatus` → move → `Stop` (if the
library supports it cleanly), `GetImagingSettings`, and an event
subscription/`PullMessages` if the library handles it without bespoke hacks. The live run
asserts each operation succeeds (the client interoperates); captured responses are
normalized + promoted as interop evidence. A client failure to complete is a real interop
`SutFail` (surfaced, never masked) — exactly the cross-impl signal interop exists to catch.

## 13. CI & phasing

- **Per-commit:** Rust **Layer 1** (replay vs frozen snapshots) in the existing CI — fast,
  no Docker. The onvif-server `cargo test --workspace` picks it up; the existing
  `NAVISTAU_READ_TOKEN`/soap-server path-dep checkout already lets CI build the crate.
- **Nightly / on-demand (new workflow, Linux + Docker):** **Layer 2** — compose up
  (oracle + onvif-srvd + zeep + our server), validate, invariants, srvd-subset diff,
  zeep interop, promote, per-scenario verdict report (surfacing still-`unverified` count).

**Phasing within Phase 2 (mirrors 1a/1b/1c):**
- **2a — Rust Layer-1 foundation:** controlled onvif-server binary + deterministic fixture
  + scenario set + path-scoped masks + replay/diff; `unverified` baselines. No Docker.
- **2b — Docker Layer-2 conformance:** oracle ONVIF schema bundle + the §6 onvif-srvd
  subset + schema-validity + invariants + promotion.
- **2c — Interop:** the python-onvif-zeep Profile-S client + promotion of interop traces.

Each sub-phase gets its own implementation plan (spec→plan→build), as Phase 1 did.

## 14. Caveats & risks

- **onvif-srvd pinning:** matching onvif-srvd's device config to our fixture for the §6
  subset may still leave benign structural differences. These default to
  `ReferenceDisagreement` (non-green) and only become `KnownDivergence` (green) via an
  explicit recorded justification entry per the §5 single rule — never auto/silently. If
  onvif-srvd proves unsuitable for an op, drop it from the §6 subset (set
  `reference_mode = none`; oracle + invariants still gate that op).
- **WSDL→XSD extraction:** the embedded-schema extraction (§4) must preserve imports;
  ONVIF schemas are large and cross-import heavily — the resolver must be exhaustive
  (offline). This is the main 2b implementation risk.
- **Zeep coverage limits:** if zeep can't drive a particular op cleanly (e.g. events), that
  op is dropped from the interop sequence with a logged note — not faked.
- **Official ONVIF Device Test Tool** remains a Windows-GUI **manual** pre-release gate
  (not automatable), unchanged from the Phase 1 design.
- **Determinism on our side:** any volatile field not pinned by the §7 fixture and not in
  the §8 mask table will cause snapshot churn — the fixture must be complete.

## 15. Success criteria (Phase 2)

1. `onvif-server/crossref/` exists as a `publish = false` workspace member; onvif-server's
   `cargo package` is unaffected — `cargo package --list -p onvif-server` contains **zero
   `crossref/` files**.
2. Layer 1 runs in per-commit CI (no Docker), diffs every scenario response against the
   snapshot corpus, reports the still-`unverified` count.
3. Layer 2 brings up the oracle (ONVIF schema bundle) + onvif-srvd + our server, applies
   schema validity + semantic invariants to every scenario, runs the §6 onvif-srvd
   subset comparison, and **promotes** `unverified`→`verified`.
4. python-onvif-zeep completes its Profile-S sequence against our server (interop green).
5. Every §10 scenario reaches a §5.7 verdict; no scenario left `unverified` or
   `harness-error` in a green Phase-2 run; the round-2 regression invariants (WhiteBalance,
   PTZ coercion) are asserted.
6. All masks are path-scoped (no value-pattern masks); the deterministic fixture is
   complete enough that only the §8 table is masked.
7. The harness goes RED (non-zero exit, accurate fail/disagreement totals) on a real
   schema-invalidity, invariant violation, or reference disagreement — not a false green.
