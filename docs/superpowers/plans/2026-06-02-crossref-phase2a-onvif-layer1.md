# crossref Phase 2a — onvif-server Rust Layer-1 foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Stand up the `crossref` harness as a `publish = false` workspace member in
onvif-server with the Phase-1 Rust core ported in, an `OnvifServer::into_router()` so the
full SOAP/auth/routing stack is drivable in-process, a fully deterministic controlled
fixture, the §5a scenario-metadata contract (incl. stateful `[[steps]]` + discovery
escape hatch), named invariant/mask registries, and a Layer-1 replay/diff harness that
captures `unverified` baselines for the §10 ONVIF scenario set. No Docker.

**Architecture:** Port the proven soap-server crossref Rust modules (`normalize.rs`,
`snapshot.rs`) verbatim; extend `scenario.rs` for the ONVIF metadata contract; add an
`OnvifServer::into_router()` (additive, non-breaking — `run()` delegates to it); build a
controlled `OnvifServer` from a fixture struct implementing all five service traits with
pinned values; drive scenarios in-process via `axum_test::TestServer` over `into_router()`;
apply named structural invariants on the raw response (before masks), then mask +
prefix-canonicalize and diff against frozen `unverified` snapshots.

**Tech Stack:** Rust, `onvif-server` (the crate under test), `soap-server` (path dep, via
onvif-server), `axum`/`axum-test`, `quick-xml`, `serde`+`toml`, `similar-asserts`.

**Spec:** `docs/superpowers/specs/2026-06-02-crossref-phase2-onvif-design.md` — 2a covers
§2 packaging, §3 Layer-1 + `into_router`, §5a scenario contract (+ steps + discovery), §7
fixture, §8 masks, the §5.2 invariants, §10 scenarios as `unverified` baselines, §13 2a.
Layer-2 (oracle/onvif-srvd/promotion) and interop are Phases 2b/2c.

---

## File Structure

- `Cargo.toml` (modify) — add `[workspace] members = ["crossref"]` + `exclude = ["/crossref"]` in `[package]`.
- `src/server.rs` (modify) — extract `pub fn OnvifServer::into_router(self) -> Result<axum::Router, RunError>`; `run()` delegates.
- `crossref/Cargo.toml` (create) — `publish = false`; deps onvif-server (path `..`), soap-server (path `../../soap-server`), quick-xml, serde, toml; dev-deps tokio, axum-test, similar-asserts, axum.
- `crossref/src/lib.rs` (create) — module decls.
- `crossref/src/normalize.rs` (create) — PORTED verbatim from `../soap-server/crossref/src/normalize.rs` (MaskRule, AttrMaskRule, normalize, mask_only, canonicalize_prefixes).
- `crossref/src/snapshot.rs` (create) — PORTED verbatim from `../soap-server/crossref/src/snapshot.rs` (SnapshotStore, Status, write_unverified/verified/canonical).
- `crossref/src/scenario.rs` (create) — the §5a metadata contract (Scenario, Step, capture/inject, discovery fields). NEW.
- `crossref/src/fixture.rs` (create) — `ControlledCamera` implementing the 5 service traits with §7 pinned values.
- `crossref/src/bin/controlled_onvif_server.rs` (create) — runnable server from the fixture (admin/admin, GetSystemDateAndTime bypass), :8080.
- `crossref/src/masks.rs` (create) — named mask registry (§8 table → `&str` → `(Vec<MaskRule>, Vec<AttrMaskRule>)`).
- `crossref/src/invariants.rs` (create) — named invariant registry (`&str` → fn over the raw response bytes → `Result<(), String>`).
- `crossref/src/sut.rs` (create) — build the controlled `OnvifServer` and expose `replay`/`replay_steps` via `axum_test::TestServer` over `into_router()`.
- `crossref/scenarios/*.toml` + `*.request.xml` (create) — the §10 inventory.
- `crossref/snapshots/*.xml` + `status.toml` (generated) — `unverified` baselines.
- `crossref/tests/layer1_replay.rs` (create) — the Layer-1 harness.
- `.github/workflows/ci.yml` (modify) — ensure workspace test/clippy/fmt.
- `crossref/README.md` (create).

---

## Task 1: Workspace scaffold + port normalize.rs & snapshot.rs

**Files:** modify `Cargo.toml`; create `crossref/Cargo.toml`, `crossref/src/lib.rs`,
`crossref/src/normalize.rs`, `crossref/src/snapshot.rs`.

- [ ] **Step 1: Workspace + exclude.** In the root `Cargo.toml`, add `exclude = ["/crossref"]`
  inside `[package]` (after `description`), and append:
```toml
[workspace]
members = ["crossref"]
```

- [ ] **Step 2: `crossref/Cargo.toml`:**
```toml
[package]
name = "onvif-crossref"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
onvif-server = { path = ".." }
soap-server = { path = "../../soap-server" }
quick-xml = "0.39"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
bytes = "1"
async-trait = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }
axum-test = "20"
axum = "0.8"
similar-asserts = "1"
```
(Match `axum`/`axum-test`/`quick-xml` versions to what onvif-server/soap-server already use — read their Cargo.tomls; adjust if they differ.)

- [ ] **Step 3: Port `normalize.rs` and `snapshot.rs` VERBATIM** from
  `/Users/jhogendorn/ws/soap-server/crossref/src/normalize.rs` and `.../snapshot.rs` into
  `crossref/src/`. These are self-contained (no soap-server-specific deps). Keep their
  in-file unit tests. This gives MaskRule/AttrMaskRule/normalize/mask_only/
  canonicalize_prefixes and SnapshotStore/Status/write_unverified/write_verified/
  write_canonical.

- [ ] **Step 4: `crossref/src/lib.rs`:**
```rust
//! onvif-crossref — differential conformance & interop harness for onvif-server.
//! Phase 2a: Layer-1 replay/diff against `unverified` baselines (no Docker).
pub mod fixture;
pub mod invariants;
pub mod masks;
pub mod normalize;
pub mod scenario;
pub mod snapshot;
pub mod sut;
```
Create empty stub files for `fixture.rs`, `invariants.rs`, `masks.rs`, `scenario.rs`,
`sut.rs` (e.g. `// filled in a later task`) so the crate compiles.

- [ ] **Step 5: Verify.** `cargo build --workspace`; `cargo test -p onvif-crossref normalize:: snapshot::` → ported tests pass. `cargo package --list -p onvif-server | grep -c '^crossref/'` → `0`.

- [ ] **Step 6: Commit.** `git add Cargo.toml crossref/Cargo.toml crossref/src/ && git commit -m "feat(crossref): scaffold onvif crossref workspace member + port normalize/snapshot"`

---

## Task 2: `OnvifServer::into_router()`

**Files:** modify `src/server.rs` (+ its tests).

`run()` (server.rs ~150–249) builds a merged `axum::Router` via the `build_service!` macro
(device + optional media/ptz/imaging/events) then spawns discovery (UDP) and binds. Extract
the router construction so it can be driven in-process. ADDITIVE, non-breaking.

- [ ] **Step 1: Write the failing acceptance tests** (in `src/server.rs` tests, or
  `tests/into_router.rs`):
```rust
#[tokio::test]
async fn into_router_serves_all_registered_services() {
    // Build a server with all 5 services (use the test/dummy service impls already in the
    // crate's tests, or minimal impls), call into_router(), drive via axum_test.
    let server = OnvifServer::builder()
        .port(0)
        .device_service(/* test impl */)
        .media_service(/* test impl */)
        .ptz_service(/* test impl */)
        .imaging_service(/* test impl */)
        .event_service(/* test impl */)
        .build().unwrap();
    let router = server.into_router().expect("into_router");
    let ts = axum_test::TestServer::new(router).unwrap();
    // Each service path must respond (POST a minimal valid SOAP request, expect 200 or a SOAP fault, NOT 404):
    for path in ["/onvif/device_service","/onvif/media_service","/onvif/ptz_service","/onvif/imaging_service","/onvif/events_service"] {
        let r = ts.post(path).content_type("application/soap+xml; charset=utf-8")
            .bytes(/* minimal GetX envelope */).await;
        assert_ne!(r.status_code().as_u16(), 404, "service not mounted: {path}");
    }
}
```
(Reuse an existing test service impl from onvif-server's `tests/` if one exists; otherwise
the minimal trait-default impls suffice.)

- [ ] **Step 2: Run → FAIL** (`into_router` doesn't exist).

- [ ] **Step 3: Extract `into_router`.** Move the router-building body of `run()` (the
  `build_service!` macro + the device build + the optional `.merge(...)` blocks, ending at
  the fully-merged `router`) into:
```rust
/// Build the merged axum Router for all registered services, WITHOUT binding a port or
/// starting the WS-Discovery UDP task. Used by `run()` and by in-process tests/harnesses.
pub fn into_router(self) -> Result<axum::Router, RunError> {
    // ... the existing build_service! macro + device + optional service merges ...
    Ok(router)
}
```
Then `run(self)` becomes: compute discovery xaddr (as today), `let router = self.into_router()?` — BUT note `run()` also needs `self.port` and the discovery xaddr AFTER consuming self. Resolve by reading `self.port`/xaddr into locals BEFORE `into_router(self)`, or have `into_router` take `&mut self`/borrow and `run` keep ownership. Simplest: `run` reads `let port = self.port;` and the discovery xaddr locals first, spawns discovery, then `let router = self.into_router()?; axum::serve(bind(port), router).await`. Keep the `#[cfg(feature="discovery")]` UDP spawn in `run()` only (NOT in into_router). Preserve all existing behavior of `run()`.

- [ ] **Step 4: Run → PASS** (all 5 services mounted). Add a second test asserting `run()`
  still compiles/binds: a `#[tokio::test]` that builds on port 0 and calls `into_router()`
  then drives one authed + one unauthed request to confirm WS-Security auth + routing run
  through the full stack (post a request with no auth to a protected op → SOAP fault; with
  the dummy service, GetSystemDateAndTime-style bypass if configured).

- [ ] **Step 5: Full gate.** `cargo test -p onvif-server` (all existing tests still pass —
  `run()` unchanged behaviorally), `cargo clippy -p onvif-server --all-targets -- -D warnings`, `cargo fmt --check`.

- [ ] **Step 6: Commit.** `git add src/server.rs tests/ && git commit -m "feat(server): OnvifServer::into_router() for in-process harness driving"`

---

## Task 3: Scenario metadata contract (§5a) + parser

**Files:** create `crossref/src/scenario.rs` (with in-file tests).

Implement the §5a contract: a `Scenario` is EITHER a single-request form OR a multi-step
`[[steps]]` form; discovery uses the non-HTTP escape hatch.

- [ ] **Step 1: Failing tests** (bottom of `scenario.rs`): parse a single-request success
  scenario; a fault scenario with `[fault]`; a discovery scenario
  (`service="discovery"`, `transport="udp_discovery"`, `schema_id="none"`, `http_method="none"`);
  and a multi-step events scenario with `[[steps]]` each having `capture`/`inject`. Assert
  the parsed fields. (Write concrete TOML literals mirroring the spec §5a examples.)

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement** the model:
```rust
//! Declarative ONVIF scenario model (spec §5a). The orchestrator routes on these EXPLICIT
//! fields — never on the scenario name.
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome { Success, Fault }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode { None, Usernametoken }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceMode { None, SrvdProjection, SrvdExact }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport { Http, UdpDiscovery }
impl Default for Transport { fn default() -> Self { Transport::Http } }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FaultExpectation { pub code: String, #[serde(default)] pub subcode: Option<String> }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Capture { pub name: String, pub path: String }
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Inject { pub name: String, pub into: String } // "header:To" | "body:<path>"

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Step {
    pub operation: String,
    pub schema_id: String,
    pub request_file: String,
    pub expected_status: u16,
    pub outcome: Outcome,
    #[serde(default)] pub capture: Vec<Capture>,
    #[serde(default)] pub inject: Vec<Inject>,
    #[serde(default)] pub invariants: Vec<String>,
    #[serde(default)] pub masks: Vec<String>,
    #[serde(default)] pub fault: Option<FaultExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub service: String,                 // device|media|imaging|ptz|events|discovery
    #[serde(default)] pub transport: Transport,
    #[serde(default)] pub auth_mode_opt: Option<AuthMode>,
    #[serde(default)] pub reference_mode: Option<ReferenceMode>,
    // single-request form (None when [[steps]] present):
    #[serde(default)] pub operation: Option<String>,
    #[serde(default)] pub schema_id: Option<String>,
    #[serde(default)] pub http_method: Option<String>,
    #[serde(default)] pub expected_status: Option<u16>,
    #[serde(default)] pub outcome: Option<Outcome>,
    #[serde(default)] pub request_file: Option<String>,
    #[serde(default)] pub invariants: Vec<String>,
    #[serde(default)] pub masks: Vec<String>,
    #[serde(default)] pub fault: Option<FaultExpectation>,
    // multi-step form:
    #[serde(default)] pub steps: Vec<Step>,
}

impl Scenario {
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> { toml::from_str(s) }
    pub fn auth_mode(&self) -> AuthMode { self.auth_mode_opt.clone().unwrap_or(AuthMode::None) }
    pub fn is_discovery(&self) -> bool { self.service == "discovery" }
    pub fn is_multistep(&self) -> bool { !self.steps.is_empty() }
}
```
> NOTE: the TOML key for auth is `auth_mode`; serde maps it to `auth_mode_opt` via
> `#[serde(rename = "auth_mode")]` — add that rename. (Or name the field `auth_mode` and add
> an `auth_mode()` accessor with a different name.) Pick one and keep tests consistent.

- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit.** `git add crossref/src/scenario.rs && git commit -m "feat(crossref): ONVIF scenario metadata contract + steps/discovery forms (spec 5a)"`

---

## Task 4: Deterministic controlled fixture + controlled-server binary

**Files:** create `crossref/src/fixture.rs`, `crossref/src/bin/controlled_onvif_server.rs`.

- [ ] **Step 1: Read the service traits** in `onvif-server/src/traits/` (DeviceService,
  MediaService, ImagingService, PTZService, EventService) to learn which methods to
  override for deterministic output, and the `generated` types (DeviceInfo etc.). The
  `examples/virtual_ptz.rs` is the reference for implementing them.

- [ ] **Step 2: Implement `ControlledCamera`** in `fixture.rs` — a `#[derive(Clone)]` struct
  implementing all five traits, returning the §7 pinned values EXACTLY:
  - Device: manufacturer `Crossref`, model `Controlled-1`, firmware `1.0.0`, serial
    `CR-0001`, hardwareId `CR-HW-1`; hostname `controlled-onvif`; scopes per §7; one network
    interface (`eth0`, MAC `00:11:22:33:44:55`, fixed IPv4 e.g. `10.0.0.10`).
  - Media: one profile token `profile_0`; video source token `vsrc_0`; vsconf `vsconf_0`;
    venc `venc_0`; stream URI `rtsp://<host>:554/stream0`; snapshot `http://<host>/snapshot0`.
  - Imaging: settings emitting exactly one `tt:WhiteBalance`.
  - PTZ: config token `ptzconf_0`, node token `ptznode_0`; status pan/tilt/zoom `0.0`,
    MoveStatus `IDLE`; one preset `preset_1`; GetServiceCapabilities with `@MoveStatus=true`.
  - Events: deterministic event properties; CreatePullPointSubscription returns a FIXED
    subscription reference address (so the steps flow is deterministic); PullMessages returns
    a fixed message set (volatile CurrentTime/TerminationTime are masked, see §8).
  `<host>` comes from the configured advertised host (the builder's `.advertised_host`).
  Where a trait method's default already emits acceptable deterministic output, keep it; only
  override what's needed for the §7 pins. If a value can't be made deterministic via the
  trait API, note it (it'll need masking).

- [ ] **Step 3: `controlled_onvif_server.rs`:**
```rust
//! Layer-2 controlled ONVIF server: the deterministic ControlledCamera on all services,
//! admin/admin, GetSystemDateAndTime auth-bypassed. Listens on 0.0.0.0:8080.
use onvif_crossref::fixture::ControlledCamera;
use onvif_server::OnvifServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cam = ControlledCamera::new();
    let server = OnvifServer::builder()
        .port(8080)
        .advertised_host("controlled-onvif:8080") // or via env for Layer-2
        .auth("admin", "admin")
        // auth_bypass for unauthenticated ops (GetSystemDateAndTime) — see builder API
        .device_service(cam.clone())
        .media_service(cam.clone())
        .ptz_service(cam.clone())
        .imaging_service(cam.clone())
        .event_service(cam)
        .build()?;
    Ok(server.run().await?)
}
```
> Check the builder for how to declare auth-bypassed operations (the `run()` source passes
> an `auth_bypass` iter to the device service). If the builder doesn't expose per-op bypass,
> note it as a small additive builder change needed (GetSystemDateAndTime must be reachable
> unauthenticated per §9).

- [ ] **Step 4: Smoke test** (in `fixture.rs` tests or `sut.rs`): build via `into_router()`
  + `axum_test`, POST `GetDeviceInformation` (authed) → assert the pinned manufacturer/model
  appear; POST `GetImagingSettings` → assert exactly one `<tt:WhiteBalance>`.

- [ ] **Step 5: gate + Commit.** `cargo test -p onvif-crossref`; clippy/fmt. `git add crossref/src/fixture.rs crossref/src/bin/controlled_onvif_server.rs && git commit -m "feat(crossref): deterministic ControlledCamera fixture + controlled server binary"`

---

## Task 5: Named mask + invariant registries

**Files:** create `crossref/src/masks.rs`, `crossref/src/invariants.rs` (with tests).

- [ ] **Step 1: `masks.rs`** — map a mask name (§8 table) to rules:
```rust
use crate::normalize::{MaskRule, AttrMaskRule};
/// Resolve a named mask (spec §8) to its path-scoped rules.
pub fn resolve(name: &str) -> (Vec<MaskRule>, Vec<AttrMaskRule>) {
    match name {
        "wsa_message_id" => (vec![MaskRule::new("Envelope/Header/MessageID")], vec![]),
        "current_time"   => (vec![MaskRule::new("Envelope/Body/PullMessagesResponse/CurrentTime")], vec![]),
        "termination_time" => (vec![MaskRule::new("Envelope/Body/PullMessagesResponse/TerminationTime"),
                                    MaskRule::new("Envelope/Body/CreatePullPointSubscriptionResponse/TerminationTime")], vec![]),
        "system_datetime" => (vec![/* path to GetSystemDateAndTimeResponse UTC time fields */], vec![]),
        "host_authority" => (vec![], vec![/* AttrMaskRule(s) for host:port in XAddrs/URIs as needed */]),
        _ => (vec![], vec![]),
    }
}
pub fn resolve_all(names: &[String]) -> (Vec<MaskRule>, Vec<AttrMaskRule>) {
    let mut t = vec![]; let mut a = vec![];
    for n in names { let (mt, ma) = resolve(n); t.extend(mt); a.extend(ma); }
    (t, a)
}
```
Fill the exact element-paths by inspecting the actual responses the fixture emits (the
local-name paths). Add the names the §10 scenarios reference. Host/port masking: the
controlled fixture pins `<host>` to a fixed advertised host, so in Layer-1 the host is
constant and `host_authority` may be a no-op — include it but it only matters in Layer-2
(different container host). Test `resolve_all` composes.

- [ ] **Step 2: `invariants.rs`** — named structural assertions over the RAW response bytes:
```rust
/// A named structural invariant: returns Err(reason) if the raw response violates it.
pub fn check(name: &str, response: &[u8], ctx: &InvariantCtx) -> Result<(), String> {
    match name {
        "single_white_balance" => exactly_one_element(response, "WhiteBalance"),
        "ptz_move_status_attr" => attr_true(response, "Capabilities", "MoveStatus"),
        "relates_to_matches_probe" => relates_to_eq(response, &ctx.request_message_id),
        "stable_endpoint_uuid" => contains_path_text(response, "...EndpointReference/Address", &ctx.expected_endpoint),
        "scopes_match_fixture" => scopes_match(response, &ctx.expected_scopes),
        "xaddrs_escaped" => xaddrs_well_formed(response),
        "wsa_subscription_id_present" => has_subscription_id_header(response),
        _ => Err(format!("unknown invariant: {name}")),
    }
}
pub struct InvariantCtx {
    pub request_message_id: String,
    pub expected_endpoint: String,
    pub expected_scopes: Vec<String>,
}
```
Implement each helper with quick-xml (count elements by local-name; read an attribute by
local-name at an element; compare a path's text; etc.). TDD each against a small sample XML
snippet (a passing and a failing case). `single_white_balance` and `ptz_move_status_attr`
are the regression-locks — make those robust.

- [ ] **Step 3: gate + Commit.** tests pass, clippy/fmt. `git add crossref/src/masks.rs crossref/src/invariants.rs && git commit -m "feat(crossref): named mask + structural-invariant registries (spec 8 + 5.2)"`

---

## Task 6: Layer-1 replay/diff harness (single + steps + discovery)

**Files:** create `crossref/src/sut.rs`, `crossref/tests/layer1_replay.rs`.

- [ ] **Step 1: `sut.rs`** — build the controlled SUT and expose replay:
```rust
//! Builds the controlled ONVIF SUT in-process (OnvifServer::into_router + axum_test) and
//! replays scenario requests through the full SOAP/auth/routing stack.
use axum_test::TestServer;
use onvif_server::OnvifServer;
use crate::fixture::ControlledCamera;

pub struct Sut { server: TestServer }
pub struct Resp { pub status: u16, pub body: Vec<u8> }

pub fn build_controlled_sut() -> Sut {
    let cam = ControlledCamera::new();
    let server = OnvifServer::builder().port(0).advertised_host("controlled-onvif:8080")
        .auth("admin","admin")
        // auth_bypass GetSystemDateAndTime
        .device_service(cam.clone()).media_service(cam.clone()).ptz_service(cam.clone())
        .imaging_service(cam.clone()).event_service(cam).build().expect("build");
    Sut { server: TestServer::new(server.into_router().expect("router")).expect("ts") }
}
impl Sut {
    pub async fn post(&self, path: &str, body: &[u8], ct: &str) -> Resp {
        let r = self.server.post(path).content_type(ct).bytes(bytes::Bytes::copy_from_slice(body)).await;
        Resp { status: r.status_code().as_u16(), body: r.as_bytes().to_vec() }
    }
}
/// Map a service name → its mount path.
pub fn service_path(service: &str) -> &'static str {
    match service {
        "device" => "/onvif/device_service", "media" => "/onvif/media_service",
        "imaging" => "/onvif/imaging_service", "ptz" => "/onvif/ptz_service",
        "events" => "/onvif/events_service", _ => "/onvif/device_service",
    }
}
```

- [ ] **Step 2: `tests/layer1_replay.rs`** — the harness:
  - Load every `scenarios/*.toml`.
  - For HTTP scenarios: route to `service_path(scenario.service)`; replay the request
    (single or `[[steps]]`); for steps, run each in order, **capture** values from a step's
    response (path-scoped local-name path) and **inject** into a later step's request
    (`header:To` or `body:<path>`) by string-substituting a `{{name}}` placeholder in the
    request fixture before sending.
  - Assert `resp.status == expected_status` per step/scenario.
  - **Invariants run BEFORE masks:** for each declared invariant, `invariants::check(name,
    &resp.body, &ctx)` on the RAW body; fail the scenario on Err.
  - Normalize: `mask_only(&resp.body, text_masks, attr_masks)` (from `masks::resolve_all`)
    → diff vs `snapshots/<name>.xml`; capture `unverified` when absent + `CROSSREF_REGEN=1`.
  - For `service="discovery"` scenarios: do NOT POST. Call the pure discovery helpers
    (`onvif_server::discovery_is_probe`, `discovery_build_probe_match`) on the request
    fixture, then run the discovery invariants (relates_to, stable endpoint uuid, scopes,
    xaddrs escaped) + the non-Probe→no-response negative (`discovery_is_probe(non_probe) ==
    false`). Snapshot the built ProbeMatch (with MessageID masked).
  - For auth scenarios: `auth_mode = "usernametoken"` requests carry a WS-Security header
    (unique nonce + correct digest for admin/admin); `auth_mode="none"` requests omit it.
  Use `#[tokio::test]`.

- [ ] **Step 3: Smoke** with one or two scenarios authored inline first (e.g.
  `device_get_device_information_authed`) to prove the harness end-to-end, then Task 7
  authors the full set.

- [ ] **Step 4: gate + Commit.** `git add crossref/src/sut.rs crossref/tests/layer1_replay.rs && git commit -m "feat(crossref): Layer-1 replay/diff harness (single + steps + discovery + auth)"`

---

## Task 7: Author the §10 scenario set + capture baselines

**Files:** create `crossref/scenarios/*.toml` + `*.request.xml`; generate `crossref/snapshots/`.

- [ ] **Step 1: Author scenarios** per §10 (each with the §5a metadata). Group by service;
  set `reference_mode` per §6 (`srvd_exact` for GetDeviceInformation; `srvd_projection` for
  GetCapabilities/GetServices/(GetProfiles conditional); `none` otherwise), `schema_id` per
  service (`device-body`/`media-body`/`imaging-body`/`ptz-body`/`events-body`),
  `auth_mode` (most `usernametoken`; GetSystemDateAndTime `none`), and `invariants`/`masks`
  by name. Required negatives: PTZ malformed-coordinate fault; events unknown-subscription
  fault; auth missing/bad-auth fault; discovery non-Probe. Events PullMessages uses the
  `[[steps]]` flow. (Full list: Device 7, Media 6, Imaging 1, PTZ 9+caps, Events 4,
  Discovery 2, plus auth variants.)

- [ ] **Step 2: Author each `*.request.xml`** — a valid SOAP 1.2 ONVIF request for the op
  (authed ones carry a WS-Security UsernameToken with a UNIQUE nonce + correct admin/admin
  digest — compute via `soap_server::compute_digest`; never share a nonce). Steps requests
  use `{{name}}` placeholders for injected values.

- [ ] **Step 3: Capture baselines.** `CROSSREF_REGEN=1 cargo test -p onvif-crossref --test layer1_replay`. Then `cargo test -p onvif-crossref --test layer1_replay` (no regen) → all pass.

- [ ] **Step 4: Sanity** — confirm the WhiteBalance snapshot has exactly one `tt:WhiteBalance`;
  the PTZ malformed-coord snapshot is a fault; the discovery snapshot is a ProbeMatch with a
  masked MessageID + the stable endpoint UUID present.

- [ ] **Step 5: Commit.** `git add crossref/scenarios crossref/snapshots && git commit -m "feat(crossref): author §10 ONVIF scenarios as unverified Layer-1 baselines"`

---

## Task 8: CI wiring + README

**Files:** modify `.github/workflows/ci.yml`; create `crossref/README.md`.

- [ ] **Step 1: CI.** Ensure the test/check jobs cover the workspace: `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. If
  the existing jobs lack `--workspace`/`--all`, add them (onvif-crossref is clippy/fmt-clean).
  Layer-1 replay runs in per-commit CI automatically via `cargo test --workspace`.

- [ ] **Step 2: README** (`crossref/README.md`) — what Phase 2a is (Rust Layer-1, unverified
  baselines), how to run (`cargo test -p onvif-crossref --test layer1_replay`; regen with
  `CROSSREF_REGEN=1`), the scenario-metadata contract pointer, and that Layer-2
  (oracle/onvif-srvd/promotion) + interop are Phases 2b/2c.

- [ ] **Step 3: Final gate.** `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `cargo package --list -p onvif-server | grep -c '^crossref/'` == 0.

- [ ] **Step 4: Commit.** `git add .github/workflows/ci.yml crossref/README.md && git commit -m "ci(crossref): run onvif Layer-1 in CI + README"`

---

## Self-review notes (author)

- **Spec coverage:** §2 packaging (T1), §3 Layer-1 + into_router (T2, T6), §5a contract incl
  steps + discovery (T3, T6), §7 fixture (T4), §8 masks (T5), §5.2 invariants (T5), §10
  scenarios as unverified baselines (T7), §13 2a incl into_router acceptance tests (T2). 2b
  (oracle/onvif-srvd/promotion) + 2c (interop) are out of scope.
- **Ported code:** normalize.rs/snapshot.rs are copied verbatim from soap-server crossref
  (self-contained); the body-child extractor is a 2b concern (Layer-1 diffs the whole
  envelope), so not ported here — when 2b needs it, port the ns-preserving logic per spec §4
  (do NOT depend on soap_server::envelope private internals).
- **Product change:** `OnvifServer::into_router()` is additive/non-breaking; `run()`
  delegates; acceptance tests assert run() still works + all services mount + full
  auth/routing exercised.
- **Known implementer risks:** (1) the builder may need a small additive change to expose
  per-op auth-bypass for GetSystemDateAndTime (T4 NOTE); (2) exact element-paths for masks
  (T5) must be read off the fixture's real output; (3) WS-Security request fixtures need
  unique nonces + correct digests (T7) — the Phase-1c lesson.
- **Determinism:** the §7 fixture must pin everything not in the §8 mask table or Layer-1
  snapshots will churn; T4 must be complete.
