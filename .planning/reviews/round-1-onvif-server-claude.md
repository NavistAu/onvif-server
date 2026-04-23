# onvif-server — Round 1 Review (Claude)
Date: 2026-04-21
Reviewer: Claude (sonnet)

## Blockers (must fix before 0.1.0 publish)

- [BLOCK-OS-01] ONVIF WSDL/XSD files lack a LICENSE-ONVIF file — legal gate for publish
  File: wsdl/ directory (7 ONVIF-authored files)
  Impact: The `wsdl/` directory contains ONVIF-licensed files under ONVIF's own terms (verbatim redistribution allowed with copyright notice; modification prohibited). These files are NOT covered by the crate's `MIT OR Apache-2.0` license declaration. Publishing to crates.io with the default license declaration misrepresents the licensing of included files. This is a legal/compliance blocker. Confirmed by `docs/roadmap.md` "Licensing / Legal" section.
  Suggested fix: Create `LICENSE-ONVIF` file at the repo root containing ONVIF's redistribution terms (verbatim from the WSDL file headers). Add a note to README (plan 05-07) explaining that WSDL/XSD files are under ONVIF's separate license.

- [BLOCK-OS-02] `discovery.rs` has a `.unwrap()` in non-test, non-main library code
  File: src/discovery.rs:9
  Context: `let multicast_addr: Ipv4Addr = "239.255.255.250".parse().unwrap();` — parsing a hardcoded string literal. The string is a valid IPv4 multicast address so this will never panic in practice. However, `discovery.rs` is compiled as library code (not just in examples or tests), and any `.unwrap()` in library code is a quality red flag for 0.1.0 publish.
  Suggested fix: Replace with `Ipv4Addr::new(239, 255, 255, 250)` — avoids the parse entirely and is the idiomatic Rust way to construct a literal IPv4 address without unwrap.

- [BLOCK-OS-03] `OnvifServer` has `pub` fields — exposing internal state as public API
  File: src/server.rs:24-35
  Impact: `OnvifServer` has `pub port`, `pub username`, `pub password`, `pub device_service`, etc. fields. `password` being `pub` is a particular concern — it's optional but if set it represents a credential that leaks easily. Making internal server state pub also means any change to these fields is a semver-breaking change. This locks the author into this internal shape for all minor/patch releases.
  Suggested fix: Make all `OnvifServer` fields `pub(crate)`. Expose read-only accessors (`pub fn port(&self) -> u16`, etc.) for the fields consumers genuinely need to inspect after build. `password` should have no public accessor.

- [BLOCK-OS-04] `OnvifServerBuilder` has `pub` fields — same concern as BLOCK-OS-03
  File: src/server.rs:235-245
  Impact: `OnvifServerBuilder` has all fields `pub`. This means consumers can bypass the builder API and set fields directly. Again: changes to field names/types are semver-breaking. Also, `pub password: Option<String>` is exposed.
  Suggested fix: Make all `OnvifServerBuilder` fields `pub(crate)`. The builder API (method chain) is the intended public interface.

- [BLOCK-OS-05] `OnvifServer::run()` returns `Box<dyn std::error::Error + Send + Sync>` — too wide for a library API
  File: src/server.rs:50
  Impact: Returning a boxed `std::error::Error` from a library's primary runtime method forces consumers to do dynamic dispatch just to inspect the error. Additionally, there is no way for a consumer to distinguish between "failed to parse WSDL" (startup failure) vs "axum server error" (runtime failure) vs "builder validation failed" vs "address already in use". This makes error handling in production code very difficult.
  Suggested fix: Define a `RunError` enum (or extend `BuildError`) with variants for the different failure modes. This is a 0.1.0 API surface concern — changing this later is a breaking change. At minimum, add a `BuildError::Runtime(String)` catch-all, but ideally the errors are specific.

- [BLOCK-OS-06] Missing crate-level documentation — `//!` block absent from lib.rs
  File: src/lib.rs
  Impact: `lib.rs` has no `//!` module-level documentation. docs.rs will render a bare list of re-exports with no explanation of what the crate does, how to use it, or what ONVIF profile is supported. This is a release-readiness blocker — consumers encountering the crate on crates.io with no docs will not adopt it.
  Suggested fix: Add a `//!` block to lib.rs explaining: what the crate is, which ONVIF profile(s) are supported (Profile S core: Device, Media, PTZ, Imaging, Events), minimum usage example, and a link to the README/mdBook. This is plan 05-07 scope but should be tracked here.

## Non-blockers (should fix / document known limitations)

- [NB-OS-01] `build.rs` is a stub — declares rerun triggers, does nothing else
  File: build.rs
  Recommendation: Either remove it (simplest: `rm build.rs`) or document the intended future use (XSD codegen). A stub build.rs that does nothing is harmless but confusing for crate consumers who see build.rs and assume codegen is happening. Note in docs/roadmap.md this is flagged as a "can be removed" item.

- [NB-OS-02] `extract_local_name` is duplicated in every service handler file (5 files)
  File: src/service/device.rs, media.rs, ptz.rs, imaging.rs, events.rs
  Recommendation: Extract to a shared private utility in `src/service/mod.rs` or `src/util.rs`. Non-blocker but increases maintenance surface. Flagged in docs/roadmap.md.

- [NB-OS-03] `OnvifError` only has 3 variants — `NotImplemented`, `InvalidArgument`, `ActionNotSupported` — gap may cause silent catch-all behavior
  File: src/error.rs
  Recommendation: `NotImplemented` and `ActionNotSupported` both map to the same SOAP fault code and message. If a service implementor returns `NotImplemented` for an operation that is registered and dispatched, the consumer gets a `Receiver/ActionNotSupported` fault which is technically incorrect (the action IS supported, it's just not implemented). Consider renaming or adding a separate `Unimplemented` variant that maps to a different SOAP fault detail. Non-blocker but a API quality issue.

- [NB-OS-04] `not_implemented()` convenience function exported from `error.rs` — its type signature is `Result<T, OnvifError>` but its useful only as a default implementation stub
  File: src/error.rs:49
  Recommendation: This is a nice ergonomic helper for service trait default implementations. Add rustdoc explaining the intended use pattern. Currently undocumented.

- [NB-OS-05] Hardcoded values exposed in `constants.rs` are pub — creates API surface for values that may need to change
  File: src/constants.rs
  Recommendation: `PROFILE_TOKEN`, `VIDEO_SOURCE_TOKEN`, etc. are `pub const` values. If a future version needs to change token names, this is breaking. Consider whether these should be configurable via builder (e.g. `builder.profile_token("profile_0")`) rather than hardcoded constants. Non-blocker at 0.1.0 but worth flagging as a v0.2+ item.

- [NB-OS-06] Single-profile / single-video-source assumption is not surfaced in the public API
  File: src/constants.rs, src/service/media.rs (implied)
  Recommendation: The crate always serves exactly one 1920x1080 H264 profile. This is not stated in the crate-level docs or the service trait docs. Document as a known limitation in the README and rustdoc. Flagged in docs/roadmap.md.

- [NB-OS-07] Auth is optional — if no `.auth()` call is made, all WS-Security tokens are accepted
  File: src/server.rs (OnvifServerBuilder)
  Recommendation: `OnvifServerBuilder` has `username: None, password: None` as defaults. In `OnvifServer::run()`, the auth closure is `if Some(user) == username.as_deref()` — if username is None, `Some(user) == None` is always false, which causes the auth closure to return `None` for any user. In soap-server, returning `None` from the auth function means... unclear. This needs explicit documentation: "If `.auth()` is not called, no authentication is required." This is a security-relevant behavior that MUST be documented.

- [NB-OS-08] `advertised_host` defaults to `"0.0.0.0"` — wrong for real deployments
  File: src/server.rs:264
  Recommendation: The `advertised_host` (used in ONVIF XAddrs for capability responses) defaults to `"0.0.0.0"`. ONVIF clients will follow the XAddr to connect; `0.0.0.0` is not a routable address. Real deployments must always call `.advertised_host()`. This should be documented prominently — preferably as a builder-level validation (return `BuildError::MissingAdvertisedHost` if left as `0.0.0.0`). At minimum, add a `#[doc(alias = "xaddr")]` and a warning in the rustdoc.

- [NB-OS-09] `wsdl/` directory contents: ONVIF WSDL files should be confirmed as verbatim upstream
  File: wsdl/*.wsdl, wsdl/*.xsd
  Recommendation: The ONVIF license prohibits modification. Before publish, verify that the 7 files in `wsdl/` are verbatim from the ONVIF specs (not modified). The PTZ namespace fix commits (08c9a7f, db3145b) are in `src/service/` code, not in `wsdl/` itself — which is correct. This review cannot fully verify without diffing against upstream ONVIF spec files. Flag for manual verification.

- [NB-OS-10] `DeviceServiceHandler::new()` is `pub` but takes `Arc<dyn DeviceService>` directly, bypassing the builder
  File: src/service/device.rs:21
  Recommendation: All five `*ServiceHandler::new()` constructors are `pub`. Consumers can create raw handlers and wire them into soap-server without going through `OnvifServerBuilder`. This is a power-user escape hatch but is undocumented. Document or consider making `pub(crate)` if the `OnvifServerBuilder` is the intended sole construction path.

## cargo doc warnings

Not run at review time (environment constraints). Anticipated based on source inspection:
- `lib.rs` has no `//!` crate-level documentation block
- `generated/types.rs` exports many types (DeviceInfo, MediaProfile, etc.) — unclear if all have rustdoc
- `traits/` module may have incomplete rustdoc on trait methods (default impls in DeviceService are documented but MediaService, PTZService etc. need verification)

## cargo publish --dry-run note

Cannot run due to `path = "../soap-server"` dependency — this is a known blocker resolved by plan 05-10 (dep swap). Not treated as a review finding.

## Summary
6 blockers, 10 non-blockers.

Blockers prioritized for plan 05-05:
1. BLOCK-OS-01: LICENSE-ONVIF missing — legal gate (highest priority)
2. BLOCK-OS-02: discovery.rs unwrap() — one-line fix
3. BLOCK-OS-03/04: pub fields on OnvifServer + OnvifServerBuilder — API surface breaking change risk
4. BLOCK-OS-05: run() error type — define RunError enum
5. BLOCK-OS-06: Missing lib.rs crate-level docs — plan 05-07 but blocker for publish readiness

Non-blockers deferred to v0.2+ or plan 05-07 (docs pass).
