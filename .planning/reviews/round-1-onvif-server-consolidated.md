# onvif-server — Round 1 Consolidated Review
Date: 2026-04-21
Sources: round-1-onvif-server-claude.md, round-1-onvif-server-codex.md
Reviewers: Claude (sonnet), OpenAI Codex (gpt-5.3-codex v0.122.0)

## Blockers for Plan 05-05

### [BLOCK-OS-C01] LICENSE-ONVIF missing — legal gate for publish
Source: [BLOCK-OS-01 from Claude]
File: wsdl/ directory (7 ONVIF-authored files)
Description: The `wsdl/` directory contains ONVIF-licensed files under ONVIF's own terms (verbatim redistribution allowed with copyright notice; modification prohibited). Publishing to crates.io with only `license = "MIT OR Apache-2.0"` misrepresents the licensing of included files. This is the highest-priority legal blocker confirmed by `docs/roadmap.md`.
Severity: Legal/compliance release-blocker.
Fix: Create `LICENSE-ONVIF` at the repo root with ONVIF's redistribution terms (verbatim from the WSDL file headers). Add README attribution notice (plan 05-07 scope, but file must exist before publish).

### [BLOCK-OS-C02] PTZ coordinate parsing silently coerces malformed values to 0.0 — unintended movement
Source: [BLOCK-OS-CDX-01 from Codex, P1]
File: src/service/ptz.rs:299-300
Description: `.parse::<f32>().unwrap_or(0.0)` on PTZ coordinate attributes. Invalid coordinate strings (e.g. `x="abc"`) are silently treated as zero and forwarded to movement commands. On a physical PTZ camera this would trigger real movement. On a virtual device this masks client bugs.
Severity: Correctness + potential physical safety issue. Highest-severity correctness blocker.
Fix: Replace with proper error propagation: parse returns `Err`, map to `OnvifError::InvalidArgument("malformed PTZ coordinate: {val}")`, propagate as SOAP fault to caller.

### [BLOCK-OS-C03] `build()` never validates required services — `BuildError::MissingRequiredService` is unused
Source: [BLOCK-OS-CDX-02 from Codex]
File: src/server.rs:331-333
Description: `OnvifServerBuilder::build()` always returns `Ok`. Validation of required services (device_service, media_service, etc.) only happens in `run()` via `.ok_or("device_service is required...")`. This means build-time errors surface at runtime as `Box<dyn Error>` strings instead of typed `BuildError` variants. The existing `BuildError::MissingRequiredService` variant is never constructed.
Severity: API contract violation — builders should fail at build time.
Fix: In `build()`, add checks: `if self.device_service.is_none() { return Err(BuildError::MissingRequiredService("device_service".into())); }` for each required service.

### [BLOCK-OS-C04] `OnvifServer` and `OnvifServerBuilder` have all `pub` fields — API surface leaks internals
Source: [BLOCK-OS-03, BLOCK-OS-04 from Claude]
File: src/server.rs:24-35 (OnvifServer), src/server.rs:235-245 (OnvifServerBuilder)
Description: All fields are `pub` including `pub password: Option<String>`. This leaks credentials, locks the internal representation as a semver-stable contract (any field rename/type change is breaking), and allows consumers to bypass the builder API. This is the most common crate-API mistake in Rust crate publishing.
Severity: API surface blocker — cannot be fixed after 0.1.0 without a major version bump.
Fix: Make all fields `pub(crate)`. Add read-only accessors for fields consumers need (`pub fn port(&self) -> u16`, etc.). No accessor for `password`.

### [BLOCK-OS-C05] `run()` error type is `Box<dyn Error + Send + Sync>` — unusable for error handling in consumers
Source: [BLOCK-OS-05 from Claude]
File: src/server.rs:50
Description: `OnvifServer::run()` returns `Result<(), Box<dyn std::error::Error + Send + Sync>>`. Consumers cannot inspect the error type to distinguish startup failures from runtime failures. This is a library API concern — binary/main code can use boxed errors, but library APIs should expose typed errors.
Severity: API surface blocker — cannot change error type after 0.1.0 without breaking change.
Fix: Define a `RunError` enum with variants: `BuildError(String)` (WSDL parse / dispatch table failures at startup) and `Io(std::io::Error)` (bind/serve errors). Or extend `BuildError` with a `Runtime(Box<dyn Error>)` catch-all variant.

### [BLOCK-OS-C06] ImagingSettings response emits two `<tt:WhiteBalance>` elements — ONVIF schema violation
Source: [BLOCK-OS-CDX-04 from Codex]
File: src/service/imaging.rs:109-117
Description: When both `white_balance_cr_gain` and `white_balance_cb_gain` are set, two separate `<tt:WhiteBalance>` elements are emitted. ONVIF schema requires one `<tt:WhiteBalance>` element with both gain fields as children. Strict ONVIF clients (python-onvif-zeep, Synology, Home Assistant) will reject the malformed response.
Severity: ONVIF spec compliance blocker.
Fix: Emit a single `<tt:WhiteBalance>` containing both `<tt:CrGain>` and `<tt:CbGain>` as children when either is present.

### [BLOCK-OS-C07] discovery.rs `.unwrap()` on hardcoded literal parse
Source: [BLOCK-OS-02 from Claude]
File: src/discovery.rs:9
Description: `"239.255.255.250".parse().unwrap()` — the string is a valid literal so this never panics, but it violates library code quality standards.
Severity: Minor library quality blocker.
Fix: Replace with `Ipv4Addr::new(239, 255, 255, 250)` — no parse, no unwrap, idiomatic.

### [BLOCK-OS-C08] docs.yml has duplicate `on.push` YAML keys — main branch pushes do NOT trigger docs workflow
Source: [BLOCK-OS-CDX-03 from Codex] (same issue also in soap-server)
File: .github/workflows/docs.yml:4-7
Description: The `on.push` key is defined twice. YAML last-key-wins semantics means the `branches: [main]` trigger is overwritten by the `tags: ['v*']` definition. Docs/book deployment won't run on main pushes.
Severity: CI correctness blocker.
Fix: Merge both push triggers into one YAML mapping:
```yaml
on:
  push:
    branches: [main]
    tags: ['v*']
  workflow_dispatch:
```

### [BLOCK-OS-C09] Missing crate-level `//!` documentation in lib.rs
Source: [BLOCK-OS-06 from Claude]
File: src/lib.rs
Description: `lib.rs` has no `//!` crate-level documentation. docs.rs renders a blank page with only re-export lists. Consumers have no entry point for understanding what the crate does, which ONVIF profile it supports, or how to start using it.
Severity: Publish-readiness blocker.
Fix: Add `//!` block with: crate description, ONVIF Profile S coverage (Device, Media, PTZ, Imaging, Events), minimum usage example, note on WS-Security and WS-Discovery. (Plan 05-07 scope.)

## Non-blockers (document as known limitations)

- [NB-OS-C01] build.rs stub — remove or document future codegen intent [from Claude NB-OS-01 + Codex NB-OS-CDX-01]
  Recommendation: Remove build.rs. It does nothing and confuses consumers expecting codegen.

- [NB-OS-C02] `extract_local_name` duplicated in 5 service handler files [from Claude NB-OS-02]
  Recommendation: Extract to `src/service/mod.rs` shared utility. Non-blocker — DRY concern.

- [NB-OS-C03] `OnvifError` has only 3 variants — `NotImplemented` and `ActionNotSupported` map to same fault [from Claude NB-OS-03]
  Recommendation: Document the mapping. Consider `Unimplemented` vs `ActionNotSupported` distinction for v0.2.

- [NB-OS-C04] `not_implemented()` convenience function is undocumented [from Claude NB-OS-04]
  Recommendation: Add rustdoc explaining it is a helper for default trait implementations.

- [NB-OS-C05] Constants in constants.rs are pub — tokenized hardcoded values as API surface [from Claude NB-OS-05]
  Recommendation: Consider builder configuration for token names in v0.2. Document as known limitation.

- [NB-OS-C06] Single-profile / single-video-source / hardcoded resolution not surfaced in API [from Claude NB-OS-06]
  Recommendation: Document in README and rustdoc as known limitation.

- [NB-OS-C07] Auth-optional behavior undocumented — if `.auth()` not called, authentication behavior is unclear [from Claude NB-OS-07]
  Recommendation: Document: "If `.auth()` is not called, the server will require WS-Security headers but reject all requests since no credential store is configured." (Or clarify the actual behavior by inspecting server.rs auth_fn=None path.)

- [NB-OS-C08] `advertised_host` defaults to `"0.0.0.0"` — not a routable address for ONVIF XAddrs [from Claude NB-OS-08]
  Recommendation: Add builder validation warning or prominent doc note. Consider making this required (no default) in v0.2.

- [NB-OS-C09] events.wsdl references external OASIS/W3C URLs — offline build may fail if loader misses them [from Codex NB-OS-CDX-02]
  Recommendation: Document `EmbeddedWsdlLoader` coverage. List which URLs are handled.

- [NB-OS-C10] WSDL files should be confirmed as verbatim upstream ONVIF [from Claude NB-OS-09]
  Recommendation: Manual verification: diff `wsdl/*.wsdl` against ONVIF spec downloads before publish.

## Reviewer Agreements (high confidence — both Claude and Codex flagged)

- docs.yml duplicate `push:` YAML key (both flagged independently)
- `build()` validation gap (Codex found, Claude found through API surface review of pub fields)

## Reviewer-unique findings

- Codex only: PTZ coordinate `unwrap_or(0.0)` (P1) — critical safety issue Claude missed
- Codex only: ImagingSettings WhiteBalance double-element serialization bug
- Claude only: LICENSE-ONVIF legal gate (highest priority finding overall)
- Claude only: `pub` fields on OnvifServer + OnvifServerBuilder (API surface design issue)
- Claude only: `run()` error type too broad

## Decisions Required

1. **`OnvifServer::run()` error type (BLOCK-OS-C05):** Define a specific `RunError` enum, or use `Box<dyn Error>` with a deprecation note for v0.2? At 0.1.0, changing this is safe (no published consumers). Recommended: `RunError` with `Io` + `BuildError` variant.

2. **Required services in `build()` (BLOCK-OS-C03):** Which services are truly required vs optional? `device_service` is clearly required (ONVIF requires GetSystemDateAndTime). `media_service`, `ptz_service` etc. may be optional for a Device-only server. Recommended: only `device_service` is required at `build()`; others validated at `run()` based on registered WSDL operations.

3. **`build.rs` removal (NB-OS-C01):** Remove now (trivial) or leave with comments? Recommended: remove it — the comment says future codegen but there is no such plan at 0.1.0.

## Already Planned (skip — do not list as blockers)

- No README → plan 05-07
- Cargo.toml metadata (repository, keywords, categories, readme, documentation, homepage) → plan 05-08
- `path = "../soap-server"` dep → plan 05-10 (cargo publish blocked by this but it's known)
- Media2 / DeviceIO services → v0.2+ per CONTEXT.md

## Summary
Blockers: 9 | Non-blockers: 10 | Decisions: 3

**Blocker breakdown:**
- 1 blocker from Codex only (P1 critical): PTZ coordinate coercion (CDX-01)
- 2 blockers from Codex only (P2): build() validation gap (CDX-02), ImagingSettings WhiteBalance bug (CDX-04)
- 1 blocker from both reviewers: docs.yml YAML key (CDX-03 / Codex higher confidence)
- 5 blockers from Claude only: LICENSE-ONVIF, pub fields, run() error type, missing lib.rs docs, discovery.rs unwrap
