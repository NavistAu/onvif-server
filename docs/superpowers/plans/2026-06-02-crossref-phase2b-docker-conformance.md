# crossref Phase 2b — onvif Docker Layer-2 conformance + promotion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.
> **DO NOT invoke any GSD / milestone skill.** This is a code plan only; ignore the onvif `.planning/` GSD structure.

**Goal:** Stand up the Docker Layer-2 pipeline for onvif-server: a Java XML oracle carrying
the offline ONVIF schema bundle (§4), an `onvif-srvd` reference device for the narrow
read-only subset (§6), and a Rust orchestrator that validates every Layer-1 scenario
response against the ONVIF XSDs, applies the §5.2 structural invariants, runs the
`srvd_projection`/`srvd_exact` comparison where declared, assigns §5.7 verdicts, and
**promotes** the 2a `unverified` baselines to `verified`.

**Architecture:** Port the proven soap-server Layer-2 Rust core (`oracle.rs`,
`layer2/{compose,verdict,promote,report}.rs`, the ns-preserving body extractor) into
`onvif-server/crossref/`, then adapt: the oracle validates per-service `*-body` schemas
extracted offline from the ONVIF WSDLs; the verdict gains a `srvd_projection` mode; the
orchestrator drives the SAME 2a `scenarios/*.toml` (routing by their `reference_mode`/
`schema_id`/`auth_mode` metadata). The controlled SUT is the 2a `controlled_onvif_server`
binary (containerized). All Docker is driven through the `cargo`-invoked orchestrator
(sandbox-excluded → no prompts); base images use tags (digest-pinning is an operator
finalization).

**Tech Stack:** Rust (port from soap-server crossref Layer-2), Docker + compose, Java 21
(oracle: JAXP/Xerces + Santuario, containerised; ONVIF schema bundle), gSOAP `onvif-srvd`.

**Spec:** `docs/superpowers/specs/2026-06-02-crossref-phase2-onvif-design.md` — §4 oracle
schema bundle, §5 conformance model + §5.7 verdict, §6 onvif-srvd narrow subset +
projection, §8 masks (reuse 2a registry), §13 2b. Interop (zeep) is Phase 2c.

---

## File Structure
- `crossref/src/oracle.rs` (create) — PORT from soap-server crossref (HTTP client: validate, c14n).
- `crossref/src/body_extract.rs` (create) — ns-preserving body-child extractor (PORT the logic per spec §4; do NOT depend on soap_server::envelope internals).
- `crossref/src/layer2/{mod,compose,verdict,promote,report}.rs` (create) — PORT + adapt (projection mode).
- `crossref/src/projection.rs` (create) — named projection extractors (GetCapabilities/GetServices/GetProfiles per §6).
- `crossref/src/bin/layer2.rs` (create) — orchestrator CLI.
- `crossref/comparators/oracle/` (create) — Java XML oracle + ONVIF schema bundle + extractor build step.
- `crossref/comparators/onvif-srvd/` (create) — onvif-srvd Dockerfile + pinned config.
- `crossref/comparators/controlled-server.Dockerfile` (create) — containerize the 2a `controlled_onvif_server` bin.
- `crossref/docker-compose.yml` + `docker-compose.local.yml` + `manifest.toml` (create).
- `.github/workflows/layer2.yml` (create) — nightly/on-demand Docker.

---

## Task 1: Port Layer-2 Rust core + body extractor
**Files:** create `crossref/src/oracle.rs`, `crossref/src/body_extract.rs`, `crossref/src/layer2/{mod,compose,verdict,promote,report}.rs`; modify `crossref/src/lib.rs`, `crossref/Cargo.toml`.

- [ ] **Step 1:** PORT `oracle.rs`, `layer2/compose.rs`, `layer2/verdict.rs`, `layer2/promote.rs`, `layer2/report.rs` VERBATIM from `/Users/jhogendorn/ws/soap-server/crossref/src/` (adjust the compose file paths to `crossref/docker-compose.yml`). Add `reqwest`+`serde_json` deps (match soap-server crossref versions).
- [ ] **Step 2:** Create `body_extract.rs` — port the namespace-preserving body-child extraction logic (copy the algorithm from soap-server's `src/envelope.rs::extract_body_first_child` — re-emit ancestor ns decls on the child — into crossref's own module; do NOT depend on `soap_server::envelope`). Unit-test: a response with `tt:`/`tds:` declared on Envelope → extracted child carries those decls.
- [ ] **Step 3:** `lib.rs` add `pub mod oracle; pub mod body_extract; pub mod layer2; pub mod projection;` (+ stub projection.rs). `cargo build --workspace`; ported unit tests pass; clippy/fmt.
- [ ] **Step 4:** Commit `feat(crossref): port Layer-2 Rust core + ns-preserving body extractor`.

## Task 2: Verdict projection mode
**Files:** modify `crossref/src/layer2/verdict.rs`; create `crossref/src/projection.rs`.

- [ ] **Step 1:** Add `ReferenceMode`-aware comparison: `srvd_exact` (full masked C14N equality), `srvd_projection` (compare only the projection — delegate to `projection::project(op, response) -> CanonProjection`), `none` (skip srvd, oracle+invariants only). Outcome-aware (reuse the 2a/Phase-1c verdict discipline: enforce declared `outcome`/`expected_status`; `is_green()` fails on SutFail/HarnessError/ReferenceDisagreement; KnownDivergence green only via recorded justification).
- [ ] **Step 2:** `projection.rs` — named extractors per §6: `get_capabilities` (service categories + XAddr path [authority masked] + Major/Minor + required capability booleans), `get_services` (service Namespaces + XAddr path + Version), `get_profiles` (token-pinned). **Asymmetric rule (§6 + review):** ignore srvd's extra services/caps, but assert OUR advertised set EXACTLY matches the fixture (a fixture-equality check). TDD each projection on sample XML (ours vs a srvd-like variant → equal projection; our-extra-service → fail).
- [ ] **Step 3:** Unit tests for the verdict modes (incl. our-side-extra → SutFail). Commit `feat(crossref): Layer-2 verdict projection mode + ONVIF projections (spec 6)`.

## Task 3: Java XML oracle with offline ONVIF schema bundle (§4)
**Files:** create `crossref/comparators/oracle/` (port soap-server's oracle, extend with ONVIF schemas).

- [ ] **Step 1:** PORT soap-server's `comparators/oracle/` (pom.xml, Oracle.java, Dockerfile, LSResourceResolver). Keep `/c14n` + `/validate?schema=` + `/healthz`.
- [ ] **Step 2:** **Offline ONVIF schema bundle.** Add a build step (a small script or Maven plugin, run in the Docker build) that extracts the embedded `<xs:schema>` from each WSDL in `onvif-server/wsdl/` (devicemgmt→`device-body`, media→`media-body`, imaging→`imaging-body`, ptz→`ptz-body`, events→`events-body`) into standalone `.xsd` files, and copies the shared `onvif.xsd`/`common.xsd`/`ws-addr.xsd`/`soap-envelope.xsd`/`xmlmime.xsd`/`xop-include.xsd`/`wsn-*`/`wsrf-*` into the oracle resources. Register each `*-body` id; extend the resolver to resolve ALL cross-schema imports to the local vendored files (no network). Vendor the WSDLs/XSDs from `onvif-server/wsdl/` (copy at build context).
- [ ] **Step 3:** Smoke (via the orchestrator/compose later, or a direct oracle build): `validate?schema=device-body` on a real `GetDeviceInformationResponse` body child (extracted, ns-preserved) → `{"valid":true}`; a malformed body → `{"valid":false}`. (Build the oracle image through compose in Task 5; if smoke-testing standalone needs direct `docker build`, that prompts — prefer validating via the Task-6 orchestrator run.)
- [ ] **Step 4:** `SCHEMAS.md` records the bundle + extraction method. Commit `feat(crossref): ONVIF schema-bundle oracle (offline WSDL schema extraction + resolver)`.

## Task 4: onvif-srvd reference container
**Files:** create `crossref/comparators/onvif-srvd/`.

- [ ] **Step 1:** Dockerize `onvif-srvd` (gSOAP ONVIF Profile-S reference server). Find a buildable source (the `onvif-srvd` open-source project) and pin it; multi-stage build. Configure it with device info matching the 2a fixture for the §6 `srvd_exact` op (GetDeviceInformation: manufacturer/model/firmware/serial) and reachable service XAddrs.
- [ ] **Step 2:** Confirm it serves the §6 subset ops (GetDeviceInformation/GetCapabilities/GetServices/GetProfiles). If onvif-srvd can't be configured to match for an op, mark that op `reference_mode=none` (oracle+invariants only) and NOTE it.
- [ ] **Step 3:** Commit `feat(crossref): onvif-srvd reference device container`.

## Task 5: Compose topology + manifest + controlled-server image
**Files:** create `crossref/comparators/controlled-server.Dockerfile`, `crossref/docker-compose.yml`, `docker-compose.local.yml`, `manifest.toml`, root `.dockerignore` (if absent).

- [ ] **Step 1:** `controlled-server.Dockerfile` — multi-stage Rust build of the 2a `controlled_onvif_server` bin (context = repo root; `.dockerignore` excludes target/.git). Healthcheck via a tool present in the runtime image (TCP probe).
- [ ] **Step 2:** compose: `controlled-server` (:8080), `oracle` (:8081), `onvif-srvd` (its port); healthchecks; `docker-compose.local.yml` publishes ports for the host orchestrator. `manifest.toml` registers oracle (schema-oracle) + onvif-srvd (reference-server, scenarios = the §6 subset) with image tags (digest-pin = operator finalization; note it).
- [ ] **Step 3:** Bring up via the orchestrator (Task 6) — all services healthy. Commit `feat(crossref): Layer-2 compose topology + manifest + controlled-server image`.

## Task 6: Orchestrator + promotion + first conformance run
**Files:** create `crossref/src/bin/layer2.rs`; finish `crossref/src/layer2/mod.rs::run()`.

- [ ] **Step 1:** `run()` drives the 2a `scenarios/*.toml` (skip discovery/interop). Per scenario: build the request (dynamic WS-Security injection for `auth_mode=usernametoken`, reusing the 2a harness helper — factor it into a shared module both Layer-1 and Layer-2 use), POST to the controlled server; validate envelope (`soap12-envelope`) + body-child (`schema_id`, ns-preserved via `body_extract`); run `invariants`; if `reference_mode != none`, POST the same request to onvif-srvd + compare per mode (`projection`/`exact`); `verdict::evaluate`; on Pass + `--promote`, `promote` (status→verified + oracle-canonical evidence; do NOT overwrite the 2a Layer-1 snapshot bytes — status-flip + `snapshots/canonical/<name>.c14n`, per Phase-1c reconciliation). Multi-step + masks handled as in 2a.
- [ ] **Step 2:** `bin/layer2.rs` — `--promote`/`--keep-up`/`--scenarios`; `Topology::up` (down -v first, per Phase-1c teardown fix) → run → report → drop(topo) BEFORE exit. Report uses the Phase-1c honest `is_green()` + fail/disagreement totals.
- [ ] **Step 3:** **End-to-end run (drive via cargo — no prompts):** `cargo run -p onvif-crossref --bin layer2 -- --promote`. Expected: HTTP scenarios validate against ONVIF schemas + pass invariants; the §6 subset agrees with onvif-srvd (projection/exact); promote to `verified`. **Any SutFail/ReferenceDisagreement is a real finding — STOP + report (do not force green).** Likely findings to surface honestly: PTZ Stop (known unreachable from 2a), any ONVIF-schema-invalid response (real onvif-server bug), srvd projection mismatches.
- [ ] **Step 4:** Verify Layer-1 still green + `.xml` snapshots untouched; workspace tests/clippy/fmt; `cargo package --list -p onvif-server | grep -c '^crossref/'` == 0. Commit `feat(crossref): Layer-2 orchestrator — validate/invariants/srvd-compare/promote`.

## Task 7: Layer-2 CI + README + close-out
**Files:** create `.github/workflows/layer2.yml`; modify `crossref/README.md`.

- [ ] **Step 1:** `layer2.yml` (Linux+Docker, `workflow_dispatch`+nightly `schedule`, NOT push): checkout (+ soap-server sibling for the path dep, as ci.yml does), `docker compose ... up -d --build`, `cargo run -p onvif-crossref --bin layer2 -- --promote`, teardown `if: always()`, surface snapshot/status drift.
- [ ] **Step 2:** README Layer-2 section (Docker-only prereq; what `verified` means; the §6 srvd subset; deferred zeep interop = 2c).
- [ ] **Step 3:** Final gates (workspace test/clippy/fmt; package-exclude 0; yaml valid). Commit `ci(crossref): onvif Layer-2 conformance workflow + README`.

---

## Self-review notes
- **Spec coverage:** §4 oracle bundle (T3), §5 conformance + §5.7 verdict (T2,T6), §6 srvd subset + projection (T2,T4), §8 masks (reuse 2a), §13 2b (all). Interop = 2c.
- **Reuse:** ports soap-server Layer-2 core (oracle/compose/verdict/promote/report); the SAME 2a scenarios drive Layer-2 (routing by their metadata) — no scenario duplication.
- **Known risks:** (1) ONVIF WSDL embedded-schema extraction + import resolution is the main effort (T3) — ONVIF schemas cross-import heavily; the resolver must be exhaustive + offline. (2) onvif-srvd config-matching for `srvd_exact` GetDeviceInformation (T4) — if intractable, downgrade to projection or none + NOTE. (3) PTZ Stop is unreachable via SOAP (2a finding) — its scenario stays fault-outcome; surface as an operator product decision. (4) shared dynamic-WS-Security helper should be factored so Layer-1 + Layer-2 share it (T6 Step 1).
- **Safety:** Docker driven via cargo (no sandbox prompts); digest-pinning deferred (operator); nothing pushed; SutFail/disagreement surfaced not masked; Layer-1 `.xml` snapshots never overwritten by promotion.
