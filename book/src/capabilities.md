# Capabilities & Limitations

What `onvif-server` is for, what a client can expect, and where it stops. For the
exact per-operation breakdown, see [Operation Coverage](./coverage.md); this page
is the crate-level summary.

## What it is

A library for standing up a **Profile S streaming-core** ONVIF *device* server in
Rust. It bundles the official ONVIF WSDLs, runs them over the
[`soap-server`](https://crates.io/crates/soap-server) transport, and exposes a
handler trait per service. You implement only the operations your device supports;
everything else is handled by the framework, returns a sensible default, or faults.

It exists to make a camera/encoder discoverable and consumable by ONVIF clients
(Frigate, Home Assistant, ONVIF Device Manager, NVRs) — not to emulate a
fully-featured commercial ONVIF device.

## What a client gets

- **Discovery** of the device and its services (`GetCapabilities`, `GetServices`,
  and optional WS-Discovery — the server answers a `Probe` with `ProbeMatches`).
- **One or more media profiles** with a stream URI and optional snapshot URI
  (you supply the URIs).
- **PTZ control** (moves, stop, status, presets) when you implement the PTZ trait.
- **Imaging settings** readout when implemented.
- **A pull-point event subscription lifecycle** (create / pull / unsubscribe).

## Configuration surface

- **Services are opt-in.** Only the Device service is required; Media, PTZ,
  Imaging, and Events are mounted (and advertised in `GetCapabilities`/`GetServices`)
  only when you register them.
- **Auth is opt-in.** With `.auth(user, pass)`, WS-Security UsernameToken is
  enforced on all non-bypassed operations; without it the server is
  **unauthenticated**. `GetSystemDateAndTime` is always auth-exempt so clients can
  sync clocks before authenticating. See [WS-Security](./ws-security.md).
- **`advertised_host`** sets the host clients see in XAddrs (`GetCapabilities`/
  `GetServices`/discovery) — it must be an address the client can route to (not
  `0.0.0.0`). Stream/snapshot URIs are *not* derived from it; your `MediaService`
  returns those (and must point them at a routable address too).
- **WS-Discovery is behind the non-default `discovery` Cargo feature.** Enable it
  to advertise on UDP multicast; see [WS-Discovery](./discovery.md).

## Limitations

- **Profile S core only** — no Profile T/G/A/M/D/C operations.
- **Media1 only** — Media2 (`ver20/media`) is not implemented.
- **Almost no configuration writes.** The only `Set*` operation is PTZ
  `SetPreset`. Hostname, scopes, encoder config, imaging, users, and network are
  not settable.
- **No real event delivery.** The subscription lifecycle works but `PullMessages`
  never returns notifications — there is no event source.
- **Some responses are static.** Video source/encoder configurations and the PTZ
  node/config tree are canned and not driven by your trait data; they can disagree
  with the profiles you advertise. Keep them consistent for strict clients.
- **Discovery scopes are fixed** (`NetworkVideoTransmitter`) and independent of
  `DeviceService::get_scopes`.
- **`GetSystemDateAndTime` always reports UTC** with `DateTimeType=Manual`.

## Conformance

Responses are differentially validated against an ONVIF schema oracle and a
reference device in CI. See [Conformance](./conformance.md) for what was checked
and how to run the harness.
