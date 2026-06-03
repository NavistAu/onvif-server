# onvif-crossref — Phase 2a Layer-1 replay harness

Differential conformance harness for `onvif-server`. Phase 2a establishes Rust Layer-1:
in-process replay against `unverified` regression baselines. No Docker required.

## What Phase 2a is

- Workspace member (`publish = false`) of the onvif-server repo.
- Drives the full SOAP/auth/routing stack in-process via `OnvifServer::into_router()` +
  `axum_test::TestServer` — no network binding.
- 29 scenarios (device, media, imaging, ptz, events, discovery, auth variants) with
  `unverified` frozen baselines in `crossref/snapshots/`.
- Baselines capture current behaviour; promotion to `verified` happens in Phase 2b once an
  oracle reference is introduced.

## Running

```sh
# Run all Layer-1 scenarios:
cargo test -p onvif-crossref --test layer1_replay

# Regenerate (overwrite) all snapshots:
CROSSREF_REGEN=1 cargo test -p onvif-crossref --test layer1_replay

# Run the full workspace (what CI runs):
cargo test --workspace
```

## Scenario-metadata contract

Each scenario is a TOML file in `crossref/scenarios/` conforming to the §5a contract in:

    docs/superpowers/specs/2026-06-02-crossref-phase2-onvif-design.md

Key fields: `service`, `transport`, `auth_mode`, `operation`, `schema_id`,
`expected_status`, `outcome`, `invariants`, `masks`. Multi-step flows use `[[steps]]`
with `capture`/`inject` for stateful sequences (e.g. events PullMessages).
Discovery scenarios use `transport = "udp_discovery"` and are exercised via the
pure discovery helpers (no HTTP POST).

## Masks and invariants

- **Masks** (`crossref/src/masks.rs`): named rules (spec §8) that blank volatile fields
  before snapshot diffing (timestamps, nonces, message IDs, host/port in URIs).
- **Invariants** (`crossref/src/invariants.rs`): named structural assertions run on the
  raw response before masking (e.g. `single_white_balance`, `ptz_move_status_attr`,
  `relates_to_matches_probe`). A failing invariant fails the scenario regardless of
  snapshot state.

## What is NOT in Phase 2a

- **Layer-2** (oracle diff against `onvif-srvd` reference, snapshot promotion to
  `verified`) — Phase 2b.
- **Interop** (python-onvif-zeep client against the controlled server) — Phase 2c.
- Docker is not required for Phase 2a.

---

## Layer-2 (Docker conformance)

Phase 2b introduces the Docker-based conformance pipeline: a Java/Xerces ONVIF schema oracle
validates every response against the real `onvif.xsd`/`common.xsd`, and the `onvif-srvd`
reference server provides a reference-agreement check where applicable.

### Prerequisite

**Docker must be running.**  The orchestrator starts a compose topology
(`crossref/docker-compose.yml` + `crossref/docker-compose.local.yml`) that includes:

- `controlled-onvif` — the onvif-server under test (built from source).
- `onvif-oracle` — Java/Xerces schema validator HTTP service.
- `onvif-srvd` — Axis/Dahua ONVIF reference device simulator.

### Running

```sh
# Run all scenarios, promote passing ones to `verified`:
cargo run -p onvif-crossref --bin layer2 -- --promote

# Same run with the drift gate (CI mode — exits non-zero on regression or stale baseline):
cargo run -p onvif-crossref --bin layer2 -- --promote --check-drift

# Run a subset of scenarios (comma-separated names):
cargo run -p onvif-crossref --bin layer2 -- --promote --scenarios ptz_get_status,device_get_hostname

# Leave containers running after the run (for debugging):
cargo run -p onvif-crossref --bin layer2 -- --keep-up
```

### What `verified` means

A scenario promoted to `verified` means:

1. The SOAP envelope is valid against the ONVIF SOAP 1.2 schema (`soap12-envelope`).
2. The response body child is valid against its specific ONVIF schema (e.g.
   `device-wsdl`, `ptz-wsdl`, `events-wsdl`).
3. All declared scenario invariants pass (structural assertions on the raw response).
4. Where `reference_mode = "srvd_exact"`, the oracle-canonicalized body child matches
   the corresponding `onvif-srvd` response byte-for-byte after masking volatile fields.
5. The oracle-C14N canonical form is written to `crossref/snapshots/canonical/<name>.c14n`
   as durable evidence.

`reference_mode = "srvd_projection"` (GetCapabilities, GetServices, GetProfiles) uses a
structure-projecting comparison rather than byte equality — see `crossref/src/projection.rs`.

`reference_mode = "none"` scenarios are validated by oracle + invariants only; no reference
comparison is performed (see §6 below).

### onvif-srvd subset — §6 legitimate device differences

Per conformance spec §6, `onvif-srvd` is only an authority where both devices can be pinned
to equivalent output.  The Phase 2b run confirmed that `onvif-srvd` legitimately diverges from
our fixture for most operations (different tokens, different capability sets, different XAddr
structure), so those scenarios are downgraded to `reference_mode = "none"`:

- `device_get_services` — srvd emits bare-authority XAddrs, no `/onvif/...` path.
- `media_get_profiles` — srvd has different profile tokens and structure.
- `device_get_capabilities` — srvd capability set differs (also independently F-5).

The one operation kept as `srvd_exact` is `device_get_device_information_authed`: device-info
VALUES match onvif-srvd exactly and the comparison drops the SOAP `<Header/>` (present in
srvd, absent in us — a SOAP-optional, non-conformance-relevant difference).

### Current status

**Release-green: 29 scenarios all `verified`, 0 unverified, empty expected-failures baseline.**
`cargo run -p onvif-crossref --bin layer2 -- --release-green` exits 0.

All Phase 2b conformance findings were surfaced by the oracle and then FIXED (F-1..F-7 +
A-1) — see `crossref/PHASE2B-FINDINGS.md`. The `--release-green` gate fails on any non-Pass
verdict, any unverified snapshot, or a non-empty `expected-failures.toml`, so the suite is a
true release conformance gate (not merely a regression/drift gate).

### Expected-failures drift gate

`crossref/expected-failures.toml` records any known-failing scenarios with their finding ID
and one-line reason (currently EMPTY — all findings resolved).  When `--check-drift` is
passed, the orchestrator compares the actual failing set against this baseline:

- **actual == expected** → exit 0 (suite healthy: only known findings red).
- **new failure not in expected** → exit 1, prints `REGRESSION/NEW FINDING: <names>`.
- **expected failure now passes** → exit 1, prints `STALE BASELINE: <names> now pass`.

To update the baseline when a finding is fixed: remove its entry from
`crossref/expected-failures.toml` and run `--promote --check-drift` to confirm.

### CI workflow

`.github/workflows/layer2.yml` runs on `workflow_dispatch` and nightly (`0 4 * * *`).
It is NOT triggered on push/PR (Docker builds are too heavy for every commit).  After the
conformance run it checks `git diff --exit-code crossref/snapshots/status.toml
crossref/snapshots/canonical/` — any unexpected change to a verified scenario's canonical
form fails the job.

**PREREQUISITE:** The CI workflow requires `origin` soap-server (`NavistAu/soap-server`) to
contain the Phase-1 crossref changes (envelope fix + `SoapFault::with_detail_xml`).  Until
soap-server is pushed, the controlled-server build uses a stale soap-server and may fail or
misbehave.  The soap-server push is a deliberate operator decision gate.

### Deferred

- **Zeep interop** (python-onvif-zeep client against the controlled server) — Phase 2c.
