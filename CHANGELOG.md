# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-06-03

A documentation + release-tooling release; no library code changes.

### Fixed

- Docs: install docs no longer hardcode versions — installation uses `cargo add`, and the
  crate version + MSRV are surfaced via auto-updating crates.io badges. The "User guide"
  link points at the live mdBook (<https://navistau.github.io/onvif-server/>).

### Internal

- CI now denies rustdoc warnings (broken intra-doc links) and lints with `--all-features`;
  CONTRIBUTING aligned to the actual CI gates.
- First release published via crates.io Trusted Publishing (OIDC) — validates the automated
  `release/* → main` publish pipeline (0.1.0 was a manual bootstrap publish).

## [0.1.0] - 2026-06-03

### Added

- `OnvifServer` and `OnvifServerBuilder` — builder-pattern entry point for
  constructing and running the ONVIF server.
- **Device Management Service** (`DeviceService` trait) — handles
  `GetSystemDateAndTime`, `GetCapabilities`, `GetDeviceInformation`,
  `GetNetworkInterfaces`, `GetScopes`, `SetScopes`, `AddScopes`, `RemoveScopes`,
  `GetHostname`, and related Device Management operations. Required service.
- **Media Service** (`MediaService` trait) — handles media profiles, stream URIs,
  and snapshot URIs. Optional; mounted at `/onvif/media_service` when registered.
- **PTZ Service** (`PTZService` trait) — handles relative move, absolute move,
  continuous move, stop, status, and preset CRUD (get, set, goto, remove). Optional;
  mounted at `/onvif/ptz_service` when registered.
- **Imaging Service** (`ImagingService` trait) — handles imaging configuration
  including brightness, contrast, and sharpness. Optional; mounted at
  `/onvif/imaging_service` when registered.
- **Events Service** (`EventService` trait) — handles event subscriptions and
  notifications. Optional; mounted at `/onvif/events_service` when registered.
- All trait methods default to returning `OnvifError::NotImplemented`, which maps to
  a SOAP fault with `ter:ActionNotSupported` subcode. Clients receive a well-formed
  fault rather than a connection error.
- **WS-Security UsernameToken digest authentication** — enabled via `.auth(username,
  password)` on the builder. `GetSystemDateAndTime` is automatically exempt as
  required by the ONVIF specification.
- **WS-Discovery** (`discovery` feature, optional) — multicast UDP listener on
  `239.255.255.250:3702`. Responds to `Probe` messages with `ProbeMatches`
  including the device XAddr and a stable EndpointReference UUID. Requires the
  `socket2` crate.
- Low-level WS-Discovery helpers `discovery_is_probe` and
  `discovery_build_probe_match` always available (no feature gate).
- Embedded ONVIF WSDL/XSD documents (`EmbeddedWsdlLoader`) served at standard ONVIF
  WSDL paths.
- `virtual_ptz` example: a fully functional in-memory PTZ camera implementing all
  five service traits, demonstrating `Arc<Mutex<_>>` state sharing.

[Unreleased]: https://github.com/NavistAu/onvif-server/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/NavistAu/onvif-server/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/NavistAu/onvif-server/releases/tag/v0.1.0
