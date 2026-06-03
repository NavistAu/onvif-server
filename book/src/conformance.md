# Conformance

`onvif-server` responses are validated differentially against independent
authorities, not just self-checked. This page summarises what that means and how to
reproduce it.

## What was validated

A differential conformance harness (`crossref/`, a non-published workspace member)
exercises **29 scenarios** spanning the device, media, imaging, PTZ, events, and
discovery surfaces, plus auth variants. It runs in two layers:

- **Layer 1 — in-process replay.** Each scenario drives the full SOAP / auth /
  routing stack through `OnvifServer::into_router()` (no network) and diffs the
  response against a frozen snapshot. Volatile fields (timestamps, nonces, message
  IDs, host/port in URIs) are masked; named **invariants** assert structural facts
  the mask would otherwise hide (e.g. `single_white_balance`,
  `ptz_move_status_attr`, the discovery `RelatesTo` echo).
- **Layer 2 — schema oracle + reference device (Docker).** Responses are validated
  against an independent ONVIF XSD oracle (Java / Xerces) and compared against the
  `onvif-srvd` reference device. This is where snapshots are promoted from
  `unverified` to `verified`. The host needs only Docker and the Rust toolchain.

The Layer-2 run is wired as a **release-green gate**: it must report all 29
scenarios `verified`, zero unverified, with an empty disagreement baseline.

This schema validation is what surfaced (and led to fixing) several real response
bugs before release — e.g. PTZ `GetStatus` UtcTime placement, white-balance
structure, capabilities element ordering, and the SOAP-1.2 fault subcode.

## What a pass means — and doesn't

A pass means the responses are **schema-valid** and structurally agree with the
reference for the operations in the [coverage matrix](./coverage.md). It does **not**
mean full ONVIF Profile S certification, nor that absent operations behave like a
commercial device — see [Capabilities & Limitations](./capabilities.md).

## Reproducing it

```sh
# Layer 1 (no Docker): fast regression replay
cargo test -p onvif-crossref --test layer1_replay

# Layer 2 (Docker): the release-green conformance gate — must exit 0 (29/29)
cargo run -p onvif-crossref --bin layer2 -- --release-green
```

Full details, scenario contract, and mask/invariant definitions are in the harness
README: <https://github.com/NavistAu/onvif-server/blob/main/crossref/README.md>.
