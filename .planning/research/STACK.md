# Stack Research

**Domain:** Rust ONVIF device server crate
**Researched:** 2026-04-05
**Confidence:** HIGH (core stack verified against soap-server source; type strategy MEDIUM due to active ecosystem decisions)

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `soap-server` (path dep) | 0.1.0 (sibling) | SOAP transport, WSDL serving, WS-Security, axum Router | The entire SOAP layer already exists — dispatch table, envelope parsing, nonce cache, auth bypass list. onvif-server must NOT re-implement any of this. |
| `tokio` | "1" (≥1.51 LTS) | Async runtime | soap-server's SoapHandler trait is async/tokio. All service traits must match. tokio 1.x is the universal async runtime for axum-based servers. |
| `axum` | "0.8" | HTTP routing | soap-server produces an `axum::Router`. onvif-server composes additional routes (WSDL endpoints per service) into the same router via `Router::merge`. Must match soap-server's axum version exactly. |
| `async-trait` | "0.1" | Async fn in dyn traits | Service traits (`DeviceService`, `PTZService`, etc.) must be `dyn`-dispatchable — the builder stores `Arc<dyn PTZService>`. Native AFIT (stable Rust 1.75+) does not support `dyn Trait` with async methods yet. async-trait remains necessary for dynamic dispatch in 2025. |
| `yaserde` + `yaserde_derive` | "0.12" | XML (de)serialize ONVIF types | The onvif-rs/schema crate uses yaserde 0.7; yaserde is at 0.12 now. If depending on onvif-rs schema types directly, match their version. If generating own types, target 0.12. Either way, yaserde is the incumbent for ONVIF type serialization in Rust — all existing generated types use it. |
| `thiserror` | "2" | Error type derivation | soap-server already uses thiserror 2. OnvifError wraps SoapFault and adds ONVIF-specific fault codes. Use thiserror 2 to stay consistent with the dependency chain. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `uuid` | "1" (≥1.23) | Generate device serial numbers, message IDs | Always — GetDeviceInformation and WS-Discovery ProbeMatch both require unique identifiers. Use `features = ["v4"]`. |
| `chrono` | "0.4" | GetSystemDateAndTime response | Always — ONVIF mandates UTC time in GetSystemDateAndTime. soap-server already carries chrono 0.4 as a dep. |
| `socket2` | "0.5" | UDP multicast socket setup for WS-Discovery | Only when `discovery` feature is enabled. tokio::net::UdpSocket cannot configure SO_REUSEADDR or multicast membership; socket2 is the standard solution for pre-configuring a socket before handing to tokio. |
| `bytes` | "1" | Byte buffer passing to soap-server handlers | Always — `SoapHandler::handle` takes and returns `Bytes`. The XML serialization output must be wrapped as `Bytes` before returning. soap-server already carries bytes 1. |
| `quick-xml` | "0.39" | Low-level XML writing for hand-crafted responses | Optional. If any ONVIF response cannot be expressed cleanly through yaserde types, quick-xml's writer gives direct control. soap-server already carries quick-xml 0.39. Zero extra cost to add. |

### Type Definition Strategy

This is the central architectural decision for onvif-server. Two options exist:

**Option A (RECOMMENDED): Depend on lumeohq/onvif-rs schema workspace crates**

The onvif-rs repository is a workspace with pre-generated types (`schema/`, `xsd_rs/`, `wsdl_rs/` subcrates) derived from official ONVIF XSDs using xsd-parser-rs + yaserde. These types are bidirectional (YaSerialize + YaDeserialize) and have been validated against real ONVIF clients.

Dependency in Cargo.toml:
```toml
# Point at the git repo or a local clone; no crates.io release for schema subcrates
onvif-schema = { git = "https://github.com/lumeohq/onvif-rs", package = "schema" }
```

Tradeoff: The onvif-rs crate is marked "DO NOT USE YET" on crates.io (v0.0.4) and documentation is sparse (~10% documented). The schema subcrate itself is internally versioned at 0.1.0. However, the pre-generated types are the most complete existing source of ONVIF struct definitions in Rust and have been used in real integrations. The risk is upstream breaking changes or unmaintained state.

**Option B (FALLBACK): Generate types with xsd-parser (v1.5.2)**

The newer `xsd-parser` crate (v1.5.2, a rewrite of xsd-parser-rs) targets `serde` + `quick-xml` rather than yaserde. It could generate types from the bundled WSDLs in a `build.rs` step.

Tradeoff: This requires writing and validating a `build.rs` generator, choosing a stable set of ONVIF XSDs, and testing the output against Frigate. It gives full control and removes the upstream dependency, but duplicates significant work. The generated types would use serde+quick-xml instead of yaserde — fine architecturally, but incompatible with any yaserde-based types from onvif-rs.

**Decision path:** Try Option A first. If the schema subcrate types have missing fields, wrong namespace annotations, or cannot be used server-side (they were designed for a client), fall back to Option B.

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo test` with `axum-test` | Integration testing against real HTTP | soap-server dev-deps include axum-test 20. onvif-server integration tests should follow the same pattern. |
| ONVIF Device Manager (Windows GUI) | Validate ONVIF compliance | Primary test tool for device-side compliance. Connect to the running server and exercise GetCapabilities, GetProfiles, etc. Free download from onvif.org. |
| Frigate (python-onvif-zeep) | Validate Frigate autotracker compatibility | The actual downstream consumer. Run Frigate's autotracker against a virtual PTZ server instance to confirm the exact call sequence works. |
| `cargo-expand` | Debug macro output from yaserde derives | When yaserde generates unexpected XML, expanding the derive macros reveals the serialization logic. |

## Installation

```toml
# Cargo.toml [dependencies]
soap-server = { path = "../soap-server" }
tokio = { version = "1", features = ["sync"] }
async-trait = "0.1"
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["std", "clock"], default-features = false }
bytes = "1"

# Type definitions — choose one strategy:
# Option A: lumeohq/onvif-rs schema crates (git dep)
# onvif-schema = { git = "https://github.com/lumeohq/onvif-rs", package = "schema" }

# Yaserde (required alongside Option A types)
# yaserde = "0.12"
# yaserde_derive = "0.12"

# Option B: generate with xsd-parser in build.rs
# xsd-parser = "1.5"   # build-dep only

[features]
default = []
discovery = ["socket2"]

[dependencies.socket2]
version = "0.5"
features = ["all"]
optional = true
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `async-trait 0.1` for service traits | Native AFIT (no macro) | When dynamic dispatch (`Arc<dyn PTZService>`) is dropped in favor of generics. If the builder accepts `<T: PTZService>` monomorphized at compile time, native AFIT works. Not recommended here because it forces the consumer binary to be generic over every service type. |
| `yaserde` for type serialization | `quick-xml` + `serde` directly | If generating own types via xsd-parser 1.5.x (Option B). quick-xml 0.39 supports serde and is already a soap-server dependency. Would avoid the yaserde compile-time bloat. |
| lumeohq/onvif-rs schema crates | Custom hand-written types | Only for the small subset of types needed (DeviceInformation, PTZStatus, etc.) — viable for MVP but unscalable across all ONVIF operations. |
| `socket2` for WS-Discovery UDP | `tokio::net::UdpSocket` directly | tokio's UdpSocket cannot set SO_REUSEADDR before binding, which is required for multicast. socket2 is the standard bridge pattern. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `yaserde` 0.7 (onvif-rs version) | Ancient relative to current 0.12; if generating own types or using yaserde directly, use current. Only match 0.7 if forced by a direct dependency on onvif-rs schema crates that pin it. | `yaserde` 0.12 unless pinned by onvif-rs |
| `xsd-parser-rs` (old lumeohq crate) | The old crate has no crates.io release and targets yaserde 0.7. The rewrite (`xsd-parser` 1.5.x) is the successor and targets serde+quick-xml. | `xsd-parser` 1.5.x if generating types |
| Re-implementing SOAP envelope parsing | soap-server Phase 1 already handles envelope parse/serialize, dispatch, auth, WSDL serving. Any reimplementation in onvif-server is wasted work and drift risk. | Use `soap-server`'s `ServerBuilder`, `SoapHandler`, `SoapFault`, `FnHandler` |
| `reqwest` or any HTTP client crate | onvif-server is a server, not a client. No HTTP client needed. | Nothing — just axum routes from soap-server |
| `serde_xml_rs` | Inferior to quick-xml for performance, has less active development in the SOAP/ONVIF space. | `quick-xml` with `serialize` feature if serde XML is needed |
| Tokio `full` features in library | Library crates should not pull in all of tokio (macros, fs, process, signal, etc.). Only declare the features actually used. | `tokio = { version = "1", features = ["sync"] }` — runtime features come from the binary |

## What soap-server Already Provides (Do Not Re-Implement)

This mapping is critical to avoid duplication:

| onvif-server needs | soap-server provides | How to use |
|--------------------|---------------------|------------|
| SOAP envelope wrapping | `serialize_envelope()` (internal), `SoapService` pipeline | Register `FnHandler` via `ServerBuilder` |
| WS-Security auth | `validate_username_token()`, nonce cache, auth bypass set | `ServerBuilder::auth_fn()`, `auth_bypass()` |
| GetSystemDateAndTime auth exemption | `ServerBuilder::auth_bypass()` | Register `"GetSystemDateAndTime"` as bypass |
| WSDL serving at `?wsdl` | `ServerBuilder::from_wsdl_file()` or `from_wsdl_bytes()` | Point at bundled WSDLs in `wsdl/` directory |
| SOAP fault responses | `SoapFault::sender()`, `SoapFault::receiver()` | Return `Err(SoapFault)` from handlers |
| Default not-implemented fault | `SoapFault::receiver("Not implemented")` | Default trait method body |
| axum Router integration | `SoapService::into_router()` | Merge with any additional axum routes |
| dispatch by body element QName | `DispatchTable` (internal), built by `ServerBuilder` | Register handler per operation name |

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `axum 0.8` | `tokio 1.x`, `hyper 1.x` | soap-server pins axum 0.8. onvif-server must use the same major version or Router::merge will fail at type level. |
| `async-trait 0.1` | Any stable Rust ≥1.36 | Still required for `dyn` + async in 2025; native AFIT does not support dyn dispatch. |
| `yaserde 0.12` | May conflict with onvif-rs schema crates (pin yaserde 0.7) | If using onvif-rs schema types as a git dep, check their pinned yaserde version first. Cargo may resolve this but could cause compile errors if feature flags differ. Investigate before committing to Option A. |
| `quick-xml 0.39` | `serde 1.x` with `serialize` feature | soap-server already carries quick-xml 0.39. No version conflict. |
| `bytes 1` | `axum 0.8`, `tokio 1.x` | soap-server carries bytes 1. SoapHandler uses it directly. No conflict. |

## Stack Patterns by Variant

**If onvif-rs schema types work cleanly (Option A succeeds):**
- Depend on onvif-rs schema git crates
- Add `yaserde` + `yaserde_derive` matching their pinned version
- In each `FnHandler`, deserialize request using `yaserde::de::from_str(&xml)`, call the trait method, serialize response using `yaserde::ser::to_string(&resp)?`

**If onvif-rs types are broken or unmaintained (fall back to Option B):**
- Write `build.rs` invoking `xsd-parser 1.5` against bundled XSDs
- Generated types use `serde` + `quick-xml`
- Deserialize in handlers: `quick_xml::de::from_str::<RequestType>(&xml)?`
- Serialize: `quick_xml::se::to_string(&response)?`
- No yaserde dependency needed

**If WS-Discovery is needed (`discovery` feature):**
- Add `socket2 = { version = "0.5", features = ["all"], optional = true }`
- Use `socket2::Socket` to bind UDP socket with SO_REUSEADDR and join multicast group `239.255.255.250:3702`
- Convert to `tokio::net::UdpSocket` via `.into_std()` + `UdpSocket::from_std()`
- WS-Discovery Probe/ProbeMatch XML is small enough to write by hand using quick-xml writer or a static string template

## Sources

- `/Users/jhogendorn/ws/soap-server/Cargo.toml` — verified soap-server dependency versions (axum 0.8, quick-xml 0.39, tokio 1, thiserror 2, bytes 1, async-trait 0.1)
- `/Users/jhogendorn/ws/soap-server/src/handler.rs` — verified SoapHandler trait signature (Bytes in/out, async-trait)
- `/Users/jhogendorn/ws/soap-server/src/server.rs` — verified ServerBuilder API (auth_bypass, auth_fn, from_wsdl_file, from_wsdl_bytes)
- [lumeohq/onvif-rs GitHub](https://github.com/lumeohq/onvif-rs) — verified schema crate structure: pre-generated types, yaserde 0.7, xsd-macro-utils from git
- [onvif/Cargo.toml](https://github.com/lumeohq/onvif-rs/blob/main/onvif/Cargo.toml) — verified yaserde 0.7, tokio 1, async-trait 0.1 in onvif-rs
- [xsd-parser docs.rs](https://docs.rs/xsd-parser/latest/xsd_parser/) — verified v1.5.2, fork of xsd-parser-rs, supports serde+quick-xml (not yaserde), ONVIF schema support plausible but not confirmed
- [Axum 0.8 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) — verified axum 0.8 is current stable (0.9 in development on main branch)
- WebSearch: tokio 1.51 LTS — MEDIUM confidence (search result only, not official docs page)
- WebSearch: thiserror 2.0.18 latest — MEDIUM confidence (search result matches docs.rs link)
- WebSearch: uuid 1.23.0 — MEDIUM confidence
- WebSearch: async-trait still needed for dyn dispatch 2025 — HIGH confidence (multiple sources + Rust RFC status confirms dyn async not yet stable)

---
*Stack research for: Rust ONVIF device server crate*
*Researched: 2026-04-05*
