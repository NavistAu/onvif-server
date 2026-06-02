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
