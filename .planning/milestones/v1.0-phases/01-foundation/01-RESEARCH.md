# Phase 1: Foundation - Research

**Researched:** 2026-04-05
**Domain:** Rust crate scaffolding — error types, WSDL embedding, service traits skeleton, builder skeleton, token constants
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- This is a porting operation informed by the best prior art (onvif-rs, python-onvif-zeep, ONVIF spec), not a greenfield design
- Follow established patterns from the research — don't reinvent where prior art already defines the right answer

### Claude's Discretion
- Type definition strategy: Try Option A (onvif-rs schema crates) first; fall back to Option B (xsd-parser build.rs) if yaserde 0.7 vs 0.12 conflict is irreconcilable. Research recommends a 30-minute compile spike to determine viability.
- WSDL/XSD bundling: Bundle all service WSDLs upfront (devicemgmt, media, ptz, imaging, events) plus shared XSDs — they're small static files and downstream phases need them. Use `include_bytes!` for compile-time embedding.
- Builder API surface: Skeleton that compiles and accepts service registration calls. Functional wiring happens in Phase 2 when the first service (Device Management) is implemented.
- Token constants: Define all crate-level `pub const` tokens (profile, video source, PTZ node, PTZ config) from day one per research recommendation. These are defaults; consumer overridability is a Phase 2+ concern if needed.
- Error types: `OnvifError` with variants matching research (NotImplemented, InvalidArgument, ActionNotSupported), mapping to SOAP faults with `xmlns:ter` namespace per pitfall #7.
- All technical decisions (architecture, patterns, naming, module structure) follow the research findings and DESIGN.md as starting points.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INFRA-01 | Crate scaffolding with Cargo.toml, soap-server path dependency, and module structure | Cargo.toml template verified against soap-server source; module structure defined in DESIGN.md and ARCHITECTURE.md |
| INFRA-02 | OnvifError type with variants for NotImplemented, InvalidArgument, ActionNotSupported, and mapping to SOAP faults with ONVIF error namespace (`ter:`) | SoapFault struct confirmed in soap-server/src/fault.rs; `xmlns:ter` requirement confirmed via pitfall #3 and SUMMARY.md |
| INFRA-03 | Embedded WSDL/XSD loader that serves bundled official ONVIF WSDLs and schemas via soap-server's WsdlLoader trait | WsdlLoader trait signature confirmed from soap-server/src/wsdl/resolver.rs: `fn load(&self, location: &str) -> Result<Vec<u8>, WsdlError>` |
| INFRA-04 | ONVIF type definitions for all request/response structures (via onvif-rs schema crates or generated from bundled XSDs) | Option A (onvif-rs git dep) vs Option B (xsd-parser build.rs) — spike required to confirm yaserde compat; see Open Questions |
| INFRA-05 | Trait-based service API where each ONVIF service is a Rust trait with async methods; unimplemented methods return spec-compliant SOAP faults by default | async-trait requirement confirmed; `not_implemented()` pattern defined in ARCHITECTURE.md |
| INFRA-06 | Builder pattern (`OnvifServer::builder()`) for server construction with service registration, auth config, and port binding | soap-server ServerBuilder API verified from source; OnvifServer wraps it per DESIGN.md |
| INFRA-07 | WS-Security UsernameToken digest authentication delegated to soap-server, with configurable credentials via builder | soap-server handles this via `ServerBuilder::auth()` closure; verified in server.rs lines 107-113 |
| INFRA-08 | Auth exemption for GetSystemDateAndTime automatically registered (per ONVIF spec, accessible without authentication) | `ServerBuilder::auth_bypass()` API verified; method signature confirmed — accepts iterator of strings |
| INFRA-09 | Token constants for consistent profile, video source, PTZ node, and PTZ configuration tokens across all services | Prevents token inconsistency pitfall (#5 in SUMMARY.md); define as `pub const &str` at crate root |
</phase_requirements>

---

## Summary

Phase 1 creates the compilable scaffold everything else depends on: `Cargo.toml`, module skeleton, `OnvifError`, `EmbeddedWsdlLoader`, service trait stubs, `OnvifServer::builder()` skeleton, and token constants. No ONVIF service operations are implemented — only the infrastructure that Phase 2+ will fill in.

The primary implementation risk is INFRA-04 (type definitions). The project research identified a potential yaserde version conflict: onvif-rs schema crates pin yaserde 0.7, but this crate targets current yaserde (0.12). This must be probed first as a spike before committing to Option A. The spike is fast (add the git dep, attempt `cargo build`, observe whether Cargo unifies cleanly). If it fails, the fallback is Option B (xsd-parser 1.5 + build.rs), which expands Phase 1 scope slightly. All other INFRA requirements are straightforward and well-understood.

The `WsdlLoader` trait interface has been verified directly from `soap-server/src/wsdl/resolver.rs`: `fn load(&self, location: &str) -> Result<Vec<u8>, WsdlError>` — no uncertainty remains. The `SoapFault` structure and `ServerBuilder` API are also confirmed from source. This phase's non-spike work is mechanical and low-risk.

**Primary recommendation:** Execute the onvif-rs yaserde spike first (Option A probe) before writing INFRA-04 tasks, then implement all other INFRA items in dependency order: Cargo.toml → error types → token constants → WSDL files + loader → trait stubs → builder skeleton.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `soap-server` | path `../soap-server` | SOAP transport, dispatch, WS-Security, WSDL serving, axum Router | Sibling crate providing all SOAP infrastructure — must not be re-implemented |
| `tokio` | `"1"` | Async runtime | soap-server uses tokio 1; must match exactly or service trait `async fn` won't compose |
| `async-trait` | `"0.1"` | Async fn in trait objects (`Arc<dyn PTZService>`) | Native AFIT does not yet support `dyn Trait` with async methods; async-trait required for dynamic dispatch in all Rust versions as of 2025 |
| `thiserror` | `"2"` | `OnvifError` derivation | soap-server already pulls thiserror 2; consistent dep chain |
| `bytes` | `"1"` | `Bytes` type for `SoapHandler` boundary | soap-server's `SoapHandler::handle` signature is `async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault>` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chrono` | `"0.4"` | UTC timestamps in `GetSystemDateAndTime` responses | Phase 1 only references it for potential re-export; used actively in Phase 2. Already a soap-server dep so zero cost |
| `uuid` | `{ version = "1", features = ["v4"] }` | Device serial numbers, WS-Discovery message IDs | Phase 2+ active use; declare in Phase 1 Cargo.toml |
| `quick-xml` | `"0.39"` | Low-level XML writing where yaserde types are awkward | Already soap-server dep; include for `OnvifError::to_soap_fault_xml()` helper |

### Type Definition — Option A (attempt first)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `onvif-schema` | `git = "https://github.com/lumeohq/onvif-rs", package = "schema"` | Pre-generated XSD types (YaSerialize + YaDeserialize) | If yaserde version resolves cleanly in Cargo |
| `yaserde` + `yaserde_derive` | Match onvif-rs pin (0.7) or 0.12 | XML de/serialize for ONVIF types | Only with Option A; version must match schema crate's pin |

### Type Definition — Option B (fallback if Option A fails)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `xsd-parser` | `"1.5"` | Generate types from bundled XSDs in build.rs | If onvif-rs yaserde conflict cannot be resolved |
| `serde` | `"1"` | Derive on generated types (Option B uses serde not yaserde) | Option B only |
| `quick-xml` | `"0.39"` | De/serialize generated types (Option B path) | Option B only; already soap-server dep |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `async-trait 0.1` | Native AFIT | Native AFIT works for static dispatch generics but breaks `Arc<dyn Service>` — builder stores boxed trait objects, so async-trait required |
| onvif-rs schema git dep | Hand-written type stubs | Hand-written stubs work for Phase 1 scope (only a few types needed at skeleton stage); onvif-rs becomes critical in Phase 2-4 |
| yaserde for XML | quick-xml + serde | quick-xml is lighter; viable only with Option B generated types |

**Installation (base Cargo.toml, pre-spike):**
```toml
[package]
name = "onvif-server"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
soap-server = { path = "../soap-server" }
tokio = { version = "1", features = ["sync"] }
async-trait = "0.1"
thiserror = "2"
bytes = "1"
chrono = { version = "0.4", features = ["std", "clock"], default-features = false }
uuid = { version = "1", features = ["v4"] }

# Type strategy — fill in after spike (Option A or B)
# Option A: onvif-schema = { git = "https://github.com/lumeohq/onvif-rs", package = "schema" }
# Option B: xsd-parser in [build-dependencies]

[features]
default = []
discovery = ["socket2"]

[dependencies.socket2]
version = "0.5"
features = ["all"]
optional = true

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

---

## Architecture Patterns

### Recommended Project Structure
```
onvif-server/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API: re-exports OnvifServer, traits, types, errors, constants
│   ├── server.rs           # OnvifServer builder skeleton
│   ├── error.rs            # OnvifError enum + not_implemented() helper
│   ├── types.rs            # DeviceInfo struct + other crate-level value types
│   ├── constants.rs        # pub const token strings (profile, video source, PTZ node, PTZ config)
│   ├── wsdl_loader.rs      # EmbeddedWsdlLoader — serves wsdl/ bytes via WsdlLoader trait
│   └── traits/
│       ├── mod.rs           # Re-exports all service traits
│       ├── device.rs        # DeviceService trait stub
│       ├── media.rs         # MediaService trait stub
│       ├── ptz.rs           # PTZService trait stub
│       ├── imaging.rs       # ImagingService trait stub
│       └── events.rs        # EventService trait stub
├── wsdl/
│   ├── devicemgmt.wsdl
│   ├── media.wsdl
│   ├── ptz.wsdl
│   ├── imaging.wsdl
│   ├── events.wsdl
│   ├── onvif.xsd
│   ├── types.xsd
│   └── ...                  # All XSD imports from the WSDLs
└── tests/
    └── foundation.rs        # Unit tests: EmbeddedWsdlLoader, OnvifError serialization, constants
```

### Pattern 1: OnvifError → SoapFault Mapping with ter: Namespace

**What:** `OnvifError` is a `thiserror`-derived enum whose variants map to specific SOAP fault codes and ONVIF subcode URIs. When converted to `SoapFault`, the fault's `detail` field must include `xmlns:ter="http://www.onvif.org/ver10/error"` on the envelope — or it must be included at the envelope level. The cleanest approach is to produce the fault detail XML with the namespace declaration inline.

**When to use:** Every `OnvifError` conversion to `SoapFault`. The `not_implemented()` function uses this.

**Example:**
```rust
// Source: soap-server/src/fault.rs (SoapFault struct), DESIGN.md (OnvifError)
use soap_server::fault::{SoapFault, FaultCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnvifError {
    #[error("Not implemented")]
    NotImplemented,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Action not supported")]
    ActionNotSupported,
}

impl OnvifError {
    pub fn into_soap_fault(self) -> SoapFault {
        let (code, reason, subcode) = match &self {
            OnvifError::NotImplemented => (
                FaultCode::Receiver,
                "Action not supported",
                "ter:ActionNotSupported",
            ),
            OnvifError::InvalidArgument(msg) => (
                FaultCode::Sender,
                msg.as_str(),
                "ter:InvalidArgVal",
            ),
            OnvifError::ActionNotSupported => (
                FaultCode::Receiver,
                "Action not supported",
                "ter:ActionNotSupported",
            ),
        };
        // detail includes xmlns:ter so python-zeep can parse the subcode
        let detail = format!(
            r#"<ter:fault xmlns:ter="http://www.onvif.org/ver10/error"><ter:subcode>{subcode}</ter:subcode></ter:fault>"#
        );
        SoapFault::new(code, reason, Some(detail))
    }
}

pub fn not_implemented() -> Result<!, OnvifError> {
    Err(OnvifError::NotImplemented)
}
```

**Critical note:** The `SoapFault::to_xml_bytes()` in soap-server wraps the fault in an envelope without `xmlns:ter`. The ONVIF requirement is that `ter:` be in scope wherever it is used. Placing `xmlns:ter` in the `detail` element itself (as shown) is sufficient — the namespace is declared at the point of use.

### Pattern 2: EmbeddedWsdlLoader

**What:** Implements soap-server's `WsdlLoader` trait by serving bundled `include_bytes!` data keyed on the import `location` string that ONVIF WSDLs use in `<xs:import location="..."/>`.

**When to use:** Phase 1 — created here; used by every service in Phase 2+.

**Example:**
```rust
// Source: soap-server/src/wsdl/resolver.rs (WsdlLoader trait), ARCHITECTURE.md pattern
use soap_server::{WsdlLoader, WsdlError};

pub struct EmbeddedWsdlLoader;

impl WsdlLoader for EmbeddedWsdlLoader {
    fn load(&self, location: &str) -> Result<Vec<u8>, WsdlError> {
        // Strip leading path components — WSDLs use relative paths like "../common/onvif.xsd"
        let filename = location.rsplit('/').next().unwrap_or(location);
        match filename {
            "devicemgmt.wsdl" => Ok(include_bytes!("../wsdl/devicemgmt.wsdl").to_vec()),
            "media.wsdl"      => Ok(include_bytes!("../wsdl/media.wsdl").to_vec()),
            "ptz.wsdl"        => Ok(include_bytes!("../wsdl/ptz.wsdl").to_vec()),
            "imaging.wsdl"    => Ok(include_bytes!("../wsdl/imaging.wsdl").to_vec()),
            "events.wsdl"     => Ok(include_bytes!("../wsdl/events.wsdl").to_vec()),
            "onvif.xsd"       => Ok(include_bytes!("../wsdl/onvif.xsd").to_vec()),
            "types.xsd"       => Ok(include_bytes!("../wsdl/types.xsd").to_vec()),
            other => Err(WsdlError::MalformedXml(format!("Unknown import: {other}"))),
        }
    }
}
```

**Critical note:** The exact import path strings in official ONVIF WSDLs must be matched. Inspect each WSDL's `<wsdl:import>` and `<xs:import>` elements after downloading and adjust the match arms accordingly. Using `rsplit('/').next()` to strip the path prefix is a robust normalization strategy.

### Pattern 3: Token Constants

**What:** All shared string tokens (profile token, video source token, PTZ node token, PTZ config token) defined as `pub const &str` at crate level.

**When to use:** Referenced everywhere — Media `GetProfiles`, PTZ `GetNodes`, `RelativeMove`, `GetStatus`. Must be defined in Phase 1 to prevent inline-string drift across services.

**Example:**
```rust
// src/constants.rs
/// Default media profile token used across all services.
pub const PROFILE_TOKEN: &str = "profile_0";
/// Default video source token.
pub const VIDEO_SOURCE_TOKEN: &str = "video_src_0";
/// Default PTZ node token.
pub const PTZ_NODE_TOKEN: &str = "ptz_node_0";
/// Default PTZ configuration token.
pub const PTZ_CONFIG_TOKEN: &str = "ptz_cfg_0";
/// TranslationSpaceFov URI — must be byte-for-byte exact for Frigate compatibility.
pub const TRANSLATION_SPACE_FOV: &str =
    "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationSpaceFov";
```

### Pattern 4: OnvifServer Builder Skeleton

**What:** Stores `Option<Arc<dyn DeviceService>>`, etc. for each service. `build()` at skeleton stage just validates that something was registered and returns `Ok(OnvifServer)` — actual wiring to soap-server `ServerBuilder` happens in Phase 2.

**When to use:** Phase 1 provides a skeleton that compiles and accepts `.device_service()`, `.media_service()`, `.ptz_service()`, `.auth()`, `.port()` calls. The `build()` method stubs out without actually starting a server.

**Example:**
```rust
// Source: DESIGN.md builder API
pub struct OnvifServerBuilder {
    port: u16,
    // Services will be added here in Phase 2+
}

pub struct OnvifServer {
    // Populated in Phase 2
}

impl OnvifServer {
    pub fn builder() -> OnvifServerBuilder {
        OnvifServerBuilder { port: 8080 }
    }
}

impl OnvifServerBuilder {
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn auth(self, _username: &str, _password: &str) -> Self {
        // Phase 2 will store credentials and configure soap-server auth_fn
        self
    }

    pub fn build(self) -> Result<OnvifServer, BuildError> {
        Ok(OnvifServer {})
    }
}
```

**Note:** The service registration methods (`.device_service()`, `.media_service()`, etc.) need to compile but need not do anything useful in Phase 1. They accept `impl DeviceService` and store it in `Option<Arc<dyn DeviceService>>`. This validates that the trait bounds and async-trait integration compile correctly.

### Pattern 5: Service Trait Stubs

**What:** Each service trait (`DeviceService`, `MediaService`, `PTZService`, `ImagingService`, `EventService`) is defined with all required and optional methods. Optional methods have `not_implemented()` defaults. Required methods have no default — consumer must implement them.

**When to use:** Phase 1 defines the trait signatures only. The `ServiceHandler` adapters (bytes-in/bytes-out bridge) live in the same files but are added per-service in Phase 2-5.

**Note on INFRA-05 scope:** Phase 1's requirement is that the trait compiles and the `not_implemented()` default works. The actual type parameters (`GetDeviceInformationResponse`, etc.) are placeholders until INFRA-04 (type definitions) is resolved. Use `()` or `todo!()` stubs if the type spike is incomplete before trait stubs are written — the key is that `cargo build` succeeds.

### Anti-Patterns to Avoid
- **Inline token strings:** Never write `"profile_0"` in service handlers. Always reference `constants::PROFILE_TOKEN`. Divergent strings cause `NoProfile` faults in clients.
- **SoapFault without xmlns:ter in detail:** The SOAP 1.2 envelope template in soap-server does not inject `xmlns:ter`. ONVIF subcodes using `ter:` prefix require the namespace declaration at the point of use in the detail XML.
- **Re-implementing SOAP envelope handling:** soap-server owns the envelope; `onvif-server` only returns body-element XML from its handlers. Never build a full `<env:Envelope>` in a SoapHandler.
- **tokio `features = ["full"]` in library:** Library `Cargo.toml` must not declare `tokio = { features = ["full"] }`. Only declare features actually used (e.g., `["sync"]`). Full features belong in consumer binaries.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SOAP envelope parse/serialize | Custom XML parser | `soap-server` `SoapService` pipeline | Envelope parsing handles SOAP 1.1/1.2 detection, header extraction, namespace handling — nontrivial |
| WS-Security UsernameToken digest | SHA1 + base64 + nonce | `soap-server` `ServerBuilder::auth()` + `validate_username_token()` | Nonce replay cache, timestamp validation, Created format — all handled |
| Operation dispatch by QName | HashMap with XML parser | `soap-server` `DispatchTable` | Handles namespace aliases, binding lookup, auth bypass set |
| WSDL serving at `?wsdl` | Custom GET handler | `soap-server` `ServerBuilder::from_wsdl_bytes_with_loader` | Handles address rewriting, import resolution |
| XML de/serialize for ONVIF types | Custom struct serializer | `yaserde` (Option A) or `serde` + `quick-xml` (Option B) | ONVIF XML has namespace-qualified attributes (`xsi:type`, etc.) that are tricky by hand |
| Auth bypass for GetSystemDateAndTime | Custom middleware | `soap-server` `ServerBuilder::auth_bypass(["GetSystemDateAndTime"])` | Already built; bypass set is consulted per-operation in the dispatch pipeline |

**Key insight:** The entire SOAP transport already exists in soap-server. Phase 1's job is to wire ONVIF-specific concerns (service traits, fault namespacing, embedded WSDLs) onto that transport — not to build a second transport layer.

---

## Common Pitfalls

### Pitfall 1: xmlns:ter Missing from SOAP Fault Envelope
**What goes wrong:** Every ONVIF fault subcode uses the `ter:` prefix (`ter:ActionNotSupported`, `ter:InvalidArgVal`). If `xmlns:ter` is not declared in scope, python-zeep (Frigate's HTTP client) throws `XMLParseError` on any error response.

**Why it happens:** soap-server's `SoapFault::to_xml_bytes()` generates a valid SOAP 1.2 envelope but does not inject ONVIF-specific namespace declarations. The envelope template is SOAP-generic.

**How to avoid:** Declare `xmlns:ter` inside the `detail` element of every fault: `<ter:fault xmlns:ter="http://www.onvif.org/ver10/error">...`. The namespace is declared at its first use — XML-valid, ONVIF-compliant.

**Verification:** Unit test that parses the serialized `OnvifError::NotImplemented` fault XML with a strict namespace-aware parser and verifies the `ter:` prefix is resolvable.

### Pitfall 2: WSDL Import Path Mismatch in EmbeddedWsdlLoader
**What goes wrong:** soap-server calls `WsdlLoader::load(location)` with the literal string from `<xs:import location="..."/>` in the WSDL file. Official ONVIF WSDLs use relative paths like `"../common/onvif.xsd"`. If the match arm expects `"onvif.xsd"` but the WSDL says `"../common/onvif.xsd"`, the loader returns `Err` and WSDL resolution fails at startup with a `BuildError::WsdlParse` error.

**Why it happens:** Developers assume a normalized filename will be passed; the actual location string is whatever the WSDL author wrote.

**How to avoid:** After downloading official ONVIF WSDLs, inspect each `<wsdl:import>` and `<xs:import>` element and catalogue the exact `location=` strings. Handle both the full relative path and the filename-only form in the match.

**Warning signs:** `BuildError::WsdlParse("Unknown import: ../common/onvif.xsd")` at startup.

### Pitfall 3: yaserde Version Conflict (Option A)
**What goes wrong:** onvif-rs schema crates pin `yaserde = "0.7"` in their `Cargo.toml`. If onvif-server adds `yaserde = "0.12"` as a direct dep, Cargo may resolve to two different yaserde versions. This compiles but the derived `YaSerialize` impls from the 0.7 version are incompatible with the 0.12 trait — attempting to call `yaserde::ser::to_string()` on an onvif-schema type fails at compile time with a trait mismatch.

**Why it happens:** Semver incompatibility across yaserde major minor versions; Cargo resolves them as separate crates.

**How to avoid:** Run the spike: `cargo add --git ... onvif-schema`, then `cargo build`. If yaserde versions conflict, either pin onvif-server to yaserde 0.7 (matching onvif-rs) or abandon Option A and use xsd-parser (Option B). Do not proceed to implement service handlers until this is resolved.

**Warning signs:** Compile errors mentioning `YaSerialize` or `YaDeserialize` trait not satisfied; two versions of `yaserde` in `cargo tree`.

### Pitfall 4: tokio Features in Library Crate
**What goes wrong:** Declaring `tokio = { features = ["full"] }` in the library's `Cargo.toml` forces all consumers to compile the full tokio suite (fs, process, signal, macros, etc.). This inflates compile times and may conflict with consumers that run a custom tokio runtime with reduced features.

**Why it happens:** Developers copy the `tokio = { features = ["full"] }` pattern from binary crates.

**How to avoid:** Use `tokio = { version = "1", features = ["sync"] }` in the library. The runtime itself is provided by the consumer binary. Only declare features the library calls directly.

### Pitfall 5: async-trait and dyn Service Objects
**What goes wrong:** Using `async fn` in a trait without `#[async_trait]` compiles when the trait is used as a generic bound (`T: DeviceService`) but fails with "the trait `DeviceService` cannot be made into an object" when stored as `Arc<dyn DeviceService>`. The builder must store boxed trait objects to avoid making `OnvifServer` generic over every service type.

**Why it happens:** Rust's native async-in-traits (stable since 1.75) generates `impl Future` return types that are not object-safe. `async-trait` works around this by boxing the future.

**How to avoid:** Apply `#[async_trait]` to every service trait definition and every `impl` block for those traits.

---

## Code Examples

Verified patterns from official sources:

### WsdlLoader Trait (from soap-server/src/wsdl/resolver.rs)
```rust
// Source: /Users/jhogendorn/ws/soap-server/src/wsdl/resolver.rs lines 24-26
pub trait WsdlLoader: Send + Sync {
    fn load(&self, location: &str) -> Result<Vec<u8>, WsdlError>;
}
```

### SoapHandler Trait (from soap-server/src/handler.rs)
```rust
// Source: /Users/jhogendorn/ws/soap-server/src/handler.rs lines 11-13
#[async_trait]
pub trait SoapHandler: Send + Sync + 'static {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault>;
}
```

### ServerBuilder Auth Bypass (from soap-server/src/server.rs)
```rust
// Source: /Users/jhogendorn/ws/soap-server/src/server.rs lines 116-126
pub fn auth_bypass<I, S>(mut self, ops: I) -> Self
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    for op in ops {
        self.auth_bypass.insert(op.into());
    }
    self
}
```

### ServerBuilder Auth Closure (from soap-server/src/server.rs)
```rust
// Source: /Users/jhogendorn/ws/soap-server/src/server.rs lines 107-113
pub fn auth<F>(mut self, f: F) -> Self
where
    F: Fn(&str) -> Option<String> + Send + Sync + 'static,
{
    self.auth_fn = Some(Arc::new(f));
    self
}
```

### SoapFault Constructor (from soap-server/src/fault.rs)
```rust
// Source: /Users/jhogendorn/ws/soap-server/src/fault.rs lines 34-40
pub fn new(code: FaultCode, reason: impl Into<String>, detail: Option<String>) -> Self {
    Self {
        code,
        reason: reason.into(),
        detail,
    }
}
```

### ServerBuilder from_wsdl_bytes_with_loader (from soap-server/src/server.rs)
```rust
// Source: /Users/jhogendorn/ws/soap-server/src/server.rs lines 82-90
pub fn from_wsdl_bytes_with_loader(
    bytes: impl Into<Vec<u8>>,
    loader: impl WsdlLoader + 'static,
) -> Self {
    let mut builder = Self::new();
    builder.wsdl_bytes = Some(bytes.into());
    builder.custom_loader = Some(Arc::new(loader));
    builder
}
```

### SoapService::into_router (from soap-server/src/server.rs)
```rust
// Source: /Users/jhogendorn/ws/soap-server/src/server.rs lines 398-423
// Returns axum::Router — composable with Router::merge()
pub fn into_router(self) -> Router { ... }
// Usage pattern in OnvifServer::build():
// let router = device_svc.into_router()
//     .merge(media_svc.into_router())
//     .merge(ptz_svc.into_router());
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `xsd-parser-rs` (lumeohq, yaserde 0.7) | `xsd-parser` 1.5.x (rewrite, serde+quick-xml) | ~2024 | If falling back to Option B, use new crate `xsd-parser` not `xsd-parser-rs` |
| Native async fn in traits (requires generics) | `async-trait 0.1` for dyn-dispatchable traits | Still current as of 2025 | async-trait required whenever trait objects are stored in Arc |
| axum 0.7 | axum 0.8 (current stable) | Jan 2025 | soap-server pins 0.8; use exactly 0.8 |

**Deprecated/outdated:**
- `xsd-parser-rs` (the old lumeohq crate): targets yaserde 0.7, no crates.io release, superseded by `xsd-parser` 1.5.x
- `yaserde` 0.7: still required if matching onvif-rs pins; avoid as a direct dep otherwise

---

## Open Questions

1. **onvif-rs yaserde version compatibility (blocks INFRA-04)**
   - What we know: onvif-rs schema crates pin yaserde 0.7; current yaserde is 0.12; Cargo semver may or may not resolve these to two separate crates
   - What's unclear: Whether `cargo build` with both onvif-schema (git) and yaserde 0.12 (direct) produces a compile error or resolves cleanly
   - Recommendation: First task in Phase 1 is a compile spike: `cargo add --git https://github.com/lumeohq/onvif-rs onvif-schema`, run `cargo build`, observe `cargo tree` output. If clean: proceed with Option A. If not: proceed with Option B (xsd-parser 1.5 + build.rs). Do not defer this past the first wave.

2. **Exact WSDL import location strings**
   - What we know: Official ONVIF WSDLs use relative paths in `<xs:import location="..."/>` that vary by WSDL version
   - What's unclear: The exact paths in the specific WSDL files that will be downloaded (devicemgmt.wsdl, media.wsdl, ptz.wsdl, imaging.wsdl, events.wsdl)
   - Recommendation: After downloading/copying WSDLs to `wsdl/`, inspect each file's import elements and catalogue the strings before writing `EmbeddedWsdlLoader`

3. **Service trait method signatures for Phase 1 stubs (blocks INFRA-05 completeness)**
   - What we know: Request/response type names are established in DESIGN.md and FEATURES.md
   - What's unclear: The exact struct field names and whether onvif-schema types match the DESIGN.md type names
   - Recommendation: Phase 1 stubs can use `()` return types or `todo!()` until INFRA-04 type strategy is resolved; replace with real types as part of INFRA-04 spike outcome

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | none — `[dev-dependencies]` in Cargo.toml |
| Quick run command | `cargo test -p onvif-server` |
| Full suite command | `cargo test -p onvif-server` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INFRA-01 | `cargo build` succeeds on fresh checkout | build | `cargo build -p onvif-server` | ❌ Wave 0 (Cargo.toml needed) |
| INFRA-02 | `OnvifError::NotImplemented` serializes to SOAP fault with `xmlns:ter` | unit | `cargo test -p onvif-server -- test_not_implemented_fault_has_ter_namespace` | ❌ Wave 0 |
| INFRA-03 | `EmbeddedWsdlLoader` returns bytes for devicemgmt, media, ptz WSDLs | unit | `cargo test -p onvif-server -- test_embedded_wsdl_loader` | ❌ Wave 0 |
| INFRA-04 | Type definitions compile | build | `cargo build -p onvif-server` | ❌ Wave 0 (post-spike) |
| INFRA-05 | Service traits compile; `not_implemented()` default returns `OnvifError` | unit | `cargo test -p onvif-server -- test_not_implemented_returns_error` | ❌ Wave 0 |
| INFRA-06 | `OnvifServer::builder()` compiles and accepts registration calls | unit | `cargo test -p onvif-server -- test_builder_accepts_service_calls` | ❌ Wave 0 |
| INFRA-07 | WS-Security auth delegated to soap-server (compile-time wiring check) | build | `cargo build -p onvif-server` | ❌ Wave 0 |
| INFRA-08 | Auth bypass for GetSystemDateAndTime is registered | unit | `cargo test -p onvif-server -- test_auth_bypass_includes_get_system_date_and_time` | ❌ Wave 0 |
| INFRA-09 | All token constants are defined as `pub const &str` | unit | `cargo test -p onvif-server -- test_token_constants_defined` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo build -p onvif-server` (fast; catches compile regressions)
- **Per wave merge:** `cargo test -p onvif-server` (full unit suite)
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `Cargo.toml` — required before any Rust file compiles
- [ ] `src/lib.rs` — crate root
- [ ] `tests/foundation.rs` — covers INFRA-02, INFRA-03, INFRA-05, INFRA-06, INFRA-08, INFRA-09

---

## Sources

### Primary (HIGH confidence)
- `/Users/jhogendorn/ws/soap-server/src/wsdl/resolver.rs` — `WsdlLoader` trait exact signature confirmed
- `/Users/jhogendorn/ws/soap-server/src/handler.rs` — `SoapHandler` trait exact signature confirmed
- `/Users/jhogendorn/ws/soap-server/src/server.rs` — `ServerBuilder::auth()`, `auth_bypass()`, `from_wsdl_bytes_with_loader()`, `SoapService::into_router()` confirmed
- `/Users/jhogendorn/ws/soap-server/src/fault.rs` — `SoapFault`, `FaultCode` confirmed; `to_xml_bytes()` does not inject `xmlns:ter` (confirmed by reading template)
- `/Users/jhogendorn/ws/soap-server/src/lib.rs` — public exports confirmed: `ServerBuilder`, `SoapService`, `BuildError`, `FileWsdlLoader`, `SoapHandler`, `FnHandler`, `SoapFault`, `FaultCode`, `WsdlLoader`, `WsdlError`
- `/Users/jhogendorn/ws/soap-server/Cargo.toml` — dependency versions confirmed: axum 0.8, tokio 1, bytes 1, thiserror 2, async-trait 0.1, quick-xml 0.39
- `/Users/jhogendorn/ws/onvif-server/docs/DESIGN.md` — module structure, builder API, service trait signatures, token constant design
- `.planning/research/SUMMARY.md`, `STACK.md`, `ARCHITECTURE.md`, `PITFALLS.md` — project pre-research confirmed and incorporated

### Secondary (MEDIUM confidence)
- [lumeohq/onvif-rs GitHub](https://github.com/lumeohq/onvif-rs) — schema crate structure, yaserde 0.7 pin (from STACK.md which verified this)
- [xsd-parser docs.rs v1.5.2](https://docs.rs/xsd-parser/latest/xsd_parser/) — Option B fallback (from STACK.md)

### Tertiary (LOW confidence)
- ONVIF WSDL import path patterns — assumed from common ONVIF WSDL conventions; must be confirmed by reading actual downloaded files

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions confirmed from soap-server source
- Architecture: HIGH — WsdlLoader, SoapHandler, ServerBuilder API all read directly from source
- WSDL import paths: LOW — must be confirmed when WSDL files are downloaded
- Type strategy (INFRA-04): MEDIUM — Option A viability requires compile spike; Option B is known-good fallback

**Research date:** 2026-04-05
**Valid until:** 2026-05-05 (stable ecosystem; soap-server source is local, not subject to upstream drift)
