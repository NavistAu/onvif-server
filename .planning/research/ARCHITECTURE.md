# Architecture Research

**Domain:** ONVIF Device Server — Rust crate layered on SOAP transport
**Researched:** 2026-04-05
**Confidence:** HIGH (based on direct reading of soap-server source, ONVIF spec knowledge, and DESIGN.md)

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        ONVIF Client (e.g. Frigate)                  │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │ HTTP POST (SOAP 1.2)
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        soap-server (axum)                            │
│  ┌─────────────┐  ┌───────────────┐  ┌──────────────────────────┐   │
│  │  WSDL Parse │  │  WS-Security  │  │  SOAP Envelope Parse     │   │
│  │  & Dispatch │  │  (auth/nonce) │  │  (body element extract)  │   │
│  └──────┬──────┘  └───────┬───────┘  └───────────┬──────────────┘   │
│         │                 │                       │                  │
│  ┌──────▼─────────────────▼───────────────────────▼──────────────┐  │
│  │     SoapHandler::handle(body: Bytes) -> Result<Bytes, Fault>   │  │
│  └──────────────────────────────┬─────────────────────────────────┘  │
└─────────────────────────────────┼───────────────────────────────────┘
                                  │ raw XML bytes (body element only)
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        onvif-server                                  │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  OnvifServer builder                                         │    │
│  │  - Wires each ONVIF service's WsdlLoader + handler table    │    │
│  │  - Registers auth_bypass for GetSystemDateAndTime           │    │
│  │  - Produces SoapService per WSDL, merges into axum Router   │    │
│  └───────────────────────────────────────────────────────────┬─┘    │
│                                                              │       │
│  ┌───────────────┐  ┌─────────────┐  ┌────────────────────┐ │       │
│  │ ServiceRouter │  │ TypeCodec   │  │ WsdlLoader         │ │       │
│  │ (op name →    │  │ (yaserde    │  │ (embedded WSDL/XSD │ │       │
│  │  trait call)  │  │  de/serial) │  │  bytes → WsdlLoader│ │       │
│  └───────┬───────┘  └─────────────┘  └────────────────────┘ │       │
│          │                                                   │       │
│  ┌───────▼───────────────────────────────────────────────────▼─┐    │
│  │  Service Traits (async, object-safe)                          │    │
│  │  DeviceService | MediaService | PTZService | ImagingService   │    │
│  │  EventService                                                 │    │
│  └───────────────────────────────┬───────────────────────────────┘    │
└──────────────────────────────────┼──────────────────────────────────┘
                                   │ Rust trait method calls
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                  Consumer (e.g. your application)                    │
│  Implements DeviceService, MediaService, PTZService, etc.           │
│  Contains actual hardware/business logic (Reolink camera calls)     │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Notes |
|-----------|----------------|-------|
| `OnvifServer` builder | Compose per-service `SoapService` instances, wire WSDL + handlers + auth-bypass, produce a merged axum `Router` | The top-level entry point consumers call |
| `ServiceRouter` (per service) | For each ONVIF operation name, deserialize the body XML into the typed request, call the trait method, serialize the response back to XML bytes | Bridge between `SoapHandler` (bytes in/out) and the typed service trait |
| `WsdlLoader` impl | Feed the bundled WSDL/XSD bytes to `soap-server`'s `ServerBuilder::from_wsdl_bytes_with_loader` | Resolves imports from the `wsdl/` directory at crate level |
| `TypeCodec` | yaserde deserialization of request XML bytes into typed structs; yaserde serialization of response structs into XML bytes | May be inlined into `ServiceRouter` rather than a separate struct |
| Service traits (`DeviceService`, `MediaService`, `PTZService`, `ImagingService`, `EventService`) | Define the async API consumers implement; provide `not_implemented()` defaults for optional operations | Core public API surface |
| `OnvifError` / `not_implemented()` | Map trait errors to `SoapFault`; provide the default body for unimplemented methods | Ensures spec-correct faults (env:Receiver / ter:ActionNotSupported) |
| `discovery` module (feature-gated) | UDP multicast on 239.255.255.250:3702 for WS-Discovery Probe/ProbeMatch | Runs as a separate tokio task alongside the axum server |

## Recommended Project Structure

```
onvif-server/
├── src/
│   ├── lib.rs              # Public API: re-exports OnvifServer, traits, types, errors
│   ├── server.rs           # OnvifServer builder — assembles SoapService instances,
│   │                       # wires WSDL loader, merges axum routers
│   ├── error.rs            # OnvifError enum, not_implemented() helper fn
│   ├── types.rs            # DeviceInfo struct and other crate-level value types
│   ├── wsdl_loader.rs      # EmbeddedWsdlLoader — serves bundled wsdl/ bytes to
│   │                       # soap-server's WsdlLoader trait
│   ├── traits/
│   │   ├── mod.rs          # Re-exports all service traits
│   │   ├── device.rs       # DeviceService trait (+ ServiceRouter impl)
│   │   ├── media.rs        # MediaService trait (+ ServiceRouter impl)
│   │   ├── ptz.rs          # PTZService trait (+ ServiceRouter impl)
│   │   ├── imaging.rs      # ImagingService trait (+ ServiceRouter impl)
│   │   └── events.rs       # EventService trait (+ ServiceRouter impl)
│   └── discovery.rs        # WS-Discovery UDP task (feature = "discovery")
├── wsdl/
│   ├── devicemgmt.wsdl
│   ├── media.wsdl
│   ├── media2.wsdl
│   ├── ptz.wsdl
│   ├── imaging.wsdl
│   ├── events.wsdl
│   ├── onvif.xsd
│   ├── types.xsd
│   └── ...                 # All XSD imports referenced by the WSDLs
├── tests/
│   ├── frigate_compat.rs   # Replay Frigate's ONVIF call sequence against a mock impl
│   └── onvif_dm_compat.rs  # Smoke test against ONVIF Device Manager expectations
└── examples/
    └── virtual_ptz.rs      # Minimal runnable example with all traits implemented
```

### Structure Rationale

- **`traits/` with ServiceRouter inline:** Each `*Service` file owns both the trait definition and the `SoapHandler` adapter that bridges bytes-in/out to the typed trait. Keeps the coupling tight and avoids a separate routing indirection layer.
- **`wsdl_loader.rs` as a distinct module:** The embedded WSDL loader has one job (serve bytes from `include_bytes!` at given import paths). Isolating it makes it easy to swap for a file-based loader in tests.
- **`wsdl/` at crate root (not `src/`):** Standard Rust convention for non-Rust assets; referenced via `include_bytes!` or `include_str!` in build.rs or directly in `wsdl_loader.rs`.
- **`discovery.rs` feature-gated:** WS-Discovery is a tokio UDP task that must not compile into binaries that don't need it. Feature flag `discovery = ["socket2"]` isolates it cleanly.

## Architectural Patterns

### Pattern 1: Per-Service SoapHandler Adapter (ServiceRouter)

**What:** For each ONVIF service, a private struct implements `soap-server`'s `SoapHandler` trait. Its `handle(body: Bytes)` method dispatches on the XML element local name, deserializes via yaserde into the typed request struct, calls the corresponding trait method on the consumer's implementation, serializes the response, and returns `Ok(Bytes)`. The struct holds an `Arc<dyn DeviceService>` (or `MediaService`, etc.).

**When to use:** Always — this is the bridge pattern required by soap-server's raw-bytes interface.

**Trade-offs:** Boilerplate per service (match arm per operation), but it is the only type-safe way to bridge from soap-server's `SoapHandler` boundary to typed Rust calls. Macro generation can reduce boilerplate later if the operation count grows burdensome.

**Example:**

```rust
struct DeviceServiceHandler {
    inner: Arc<dyn DeviceService>,
}

#[async_trait]
impl SoapHandler for DeviceServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        // Extract operation name from the root element local name
        let op = element_local_name(&body)?;
        match op.as_str() {
            "GetDeviceInformation" => {
                // yaserde deserialization not needed for no-arg ops
                let resp = self.inner.get_device_information().await
                    .map_err(|e| e.into_soap_fault())?;
                serialize_response(resp)
            }
            "GetCapabilities" => {
                let req: GetCapabilitiesRequest = yaserde::de::from_reader(body.as_ref())
                    .map_err(|e| SoapFault::sender(e))?;
                let resp = self.inner.get_capabilities(req).await
                    .map_err(|e| e.into_soap_fault())?;
                serialize_response(resp)
            }
            _ => Err(SoapFault::receiver("ActionNotSupported")),
        }
    }
}
```

### Pattern 2: default_handler for Unimplemented Operations

**What:** Register a `default_handler` on `ServerBuilder` that returns `SoapFault::receiver("ActionNotSupported")` for any operation name not explicitly handled. This prevents soap-server from returning `UnregisteredOperation` build errors for the ~30 devicemgmt operations that the `DeviceService` trait doesn't require implementors to handle.

**When to use:** Required — ONVIF WSDLs define many more operations than any single implementation will handle. Without a `default_handler`, `soap-server` would refuse to build for unregistered operations.

**Trade-offs:** The default handler must emit a well-formed ONVIF-compliant fault (`ter:ActionNotSupported` in the `env:Receiver` code). A generic 500 would break ONVIF clients.

### Pattern 3: Embedded WsdlLoader

**What:** Implement `soap-server`'s `WsdlLoader` trait with a struct that serves WSDL/XSD bytes via `include_bytes!`. Map import location strings (e.g., `"../common/onvif.xsd"`) to the correct embedded bytes.

**When to use:** Always — this is what allows the crate to be dependency-complete without filesystem access at runtime.

**Trade-offs:** Import path resolution must match the relative paths in the WSDL `<wsdl:import>` and `<xs:import>` elements exactly. A path mismatch silently causes soap-server to fail WSDL resolution at startup.

```rust
pub struct EmbeddedWsdlLoader;

impl WsdlLoader for EmbeddedWsdlLoader {
    fn load(&self, location: &str) -> Result<Vec<u8>, WsdlError> {
        match location {
            "onvif.xsd" | "../common/onvif.xsd" => Ok(include_bytes!("../wsdl/onvif.xsd").to_vec()),
            "types.xsd" | "../common/types.xsd" => Ok(include_bytes!("../wsdl/types.xsd").to_vec()),
            // ... all imported XSD/WSDL files
            other => Err(WsdlError::MalformedXml(format!("Unknown import: {other}"))),
        }
    }
}
```

## Data Flow

### Request Flow (per ONVIF operation)

```
ONVIF Client (Frigate / ONVIF Device Manager)
    │ HTTP POST /onvif/device_service
    │ Content-Type: application/soap+xml
    │ Body: <s:Envelope><s:Header><wsse:Security>...</wsse:Security></s:Header>
    │       <s:Body><tds:GetCapabilities>...</tds:GetCapabilities></s:Body></s:Envelope>
    ▼
axum router (produced by soap-server's SoapService::into_router())
    │ Extracts raw body bytes
    ▼
soap-server pipeline:
    1. detect_soap_version() — SOAP 1.2
    2. parse_envelope() — split header children and body element bytes
    3. extract_body_qname() — QName{ns=..., local="GetCapabilities"}
    4. DispatchTable lookup — finds DeviceServiceHandler, checks auth_required
    5. If auth_required: validate_username_token(security_header, auth_fn, nonce_cache)
    6. DeviceServiceHandler::handle(body_bytes) called
    ▼
onvif-server (DeviceServiceHandler):
    7. Match "GetCapabilities"
    8. yaserde::de::from_reader(body_bytes) → GetCapabilitiesRequest
    9. DeviceService::get_capabilities(&req).await → GetCapabilitiesResponse | OnvifError
   10. yaserde::ser::to_string(resp) → XML string → Bytes
   ▼
soap-server pipeline (return path):
   11. Wrap response bytes in SOAP envelope Body
   12. Serialize full envelope to bytes
   13. HTTP 200 with Content-Type: application/soap+xml
    ▼
ONVIF Client receives response
```

### Auth Bypass Flow (GetSystemDateAndTime)

```
ONVIF Client (pre-auth clock sync, no WS-Security header)
    │ POST /onvif/device_service
    │ Body: <tds:GetSystemDateAndTime/>
    ▼
soap-server:
    dispatch entry for "GetSystemDateAndTime" has auth_required = false
    → skips WS-Security validation entirely
    → calls DeviceServiceHandler::handle() directly
    ▼
onvif-server:
    DeviceService::get_system_date_and_time().await → current UTC time response
```

### WS-Discovery Flow (feature = "discovery")

```
Network (NVR auto-discovery)
    │ UDP multicast 239.255.255.250:3702
    │ WS-Discovery Probe message
    ▼
discovery task (tokio::spawn, separate from axum):
    Parse Probe, check Types/Scopes match
    Send ProbeMatch with XAddrs pointing to onvif/device_service URL
    ▼
NVR connects via HTTP to device service URL
```

### Key Data Flows

1. **Startup wiring:** `OnvifServer::builder()` calls `soap-server`'s `ServerBuilder::from_wsdl_bytes_with_loader(wsdl_bytes, EmbeddedWsdlLoader)` for each ONVIF service, registers all operation handlers, configures auth, then calls `.build()` to produce a `SoapService`. Each `SoapService` is converted to an axum `Router` via `into_router()`, then all routers are merged.

2. **Type resolution:** soap-server resolves the WSDL at build time (startup) — parses all operations, builds the `DispatchTable`, and validates every registered handler has a matching WSDL operation. Failures here are startup errors, not runtime errors.

3. **Trait call:** The `ServiceRouter` (implementing `SoapHandler`) owns an `Arc<dyn XService>` pointing to the consumer's implementation. All ONVIF business logic stays in the consumer; the `ServiceRouter` only handles bytes-to-types-to-bytes translation.

## Scaling Considerations

This is a library crate, not a deployed service. Scaling considerations are for the downstream consumer embedding it.

| Concern | Approach |
|---------|---------|
| Multiple concurrent ONVIF clients | Handled by axum/tokio — service traits are `Send + Sync + 'static`, so `Arc<dyn XService>` is cheaply cloned to all threads |
| High-frequency GetStatus polling (Frigate calls per tracking frame) | Trait method is async; implementation controls latency. The handler layer adds negligible overhead (one yaserde de/ser round-trip) |
| Multiple simultaneous ONVIF services | `OnvifServer` merges all `SoapService` routers into one axum `Router` — no additional TCP listeners |
| WS-Discovery at scale | Not relevant — runs once at startup, low traffic |

## Anti-Patterns

### Anti-Pattern 1: One Giant SoapHandler for All Services

**What people do:** Register a single `SoapHandler` on one soap-server instance for all ONVIF operations, dispatching inside on the operation name.

**Why it's wrong:** ONVIF services have different WSDL files and different endpoint URLs (`/onvif/device_service`, `/onvif/ptz_service`, etc.). soap-server's multi-service support requires separate `SoapService` instances, each built from their respective WSDL. Collapsing everything into one handler loses the per-WSDL dispatch table and WSDL serving at `?wsdl`.

**Do this instead:** One `ServerBuilder` + `SoapService` per ONVIF WSDL, merged via axum `Router::merge()`.

### Anti-Pattern 2: Blocking Inside Trait Methods

**What people do:** Call blocking I/O (camera SDK, synchronous HTTP) directly inside `async fn get_status()`.

**Why it's wrong:** The service traits are called from within axum's async executor. Blocking the thread starves other requests.

**Do this instead:** Use `tokio::task::spawn_blocking` for unavoidably blocking operations, or use async camera SDKs. Keep trait implementations fully async.

### Anti-Pattern 3: Fat Trait (All Operations Required)

**What people do:** Define every ONVIF operation as a required trait method, forcing consumers to implement 40+ methods for devicemgmt alone.

**Why it's wrong:** Most implementations will only support a subset. Requiring all 40+ methods makes the library unusable — implementors either stub everything or skip the library.

**Do this instead:** Only a small subset of "required per spec" operations are required trait methods (no default). All others have `not_implemented()` as default. The `ServiceRouter`'s `default_handler` handles operations not listed in the trait at all.

### Anti-Pattern 4: Tight Coupling to onvif-rs Type Version

**What people do:** Expose onvif-rs types directly in public trait signatures and re-export them as part of the crate's public API.

**Why it's wrong:** If onvif-rs changes a type definition, all downstream consumers must update even if the semantic contract hasn't changed. It also locks the crate into onvif-rs if a better type source emerges.

**Do this instead:** Consider thin wrapper types or type aliases. If Option A (reuse onvif-rs) is chosen, document explicitly that onvif-rs is a semver-coupled dependency and track its releases.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| soap-server | Rust crate dependency (path dep at `~/ws/soap-server`); `ServerBuilder` + `SoapService` + `SoapHandler` + `WsdlLoader` | The entire SOAP transport, WS-Security, and WSDL parsing lives here |
| onvif-rs (lumeohq) | Crate dependency for XSD-generated types; consumed via yaserde | Decision pending: use as-is (Option A) or generate own types (Option B) |
| ONVIF clients (Frigate, ONVIF DM) | HTTP over TCP; no special integration — client contacts the axum server endpoint | Must handle clients that send SOAPAction header vs. relying purely on body element dispatch |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `OnvifServer` builder ↔ `soap-server` `ServerBuilder` | Direct Rust API calls at startup; produces `SoapService` | Happens once at startup; no runtime coupling |
| `ServiceRouter` ↔ service trait impl | `Arc<dyn XService>` — async trait method calls | The only runtime boundary between onvif-server and consumer code |
| `ServiceRouter` ↔ `soap-server` dispatch | `SoapHandler::handle(Bytes) -> Result<Bytes, SoapFault>` | The boundary that soap-server sees; completely decoupled from ONVIF semantics |
| `EmbeddedWsdlLoader` ↔ `wsdl/` directory | `include_bytes!` at compile time | No runtime filesystem access |
| `discovery` task ↔ axum server | Separate `tokio::spawn`; shares `Arc<DeviceInfo>` for XAddrs | Communication is one-way (discovery references static config) |

## Build Order Implications

The component dependency graph within onvif-server drives the recommended build order:

1. **`error.rs` and `types.rs`** — `OnvifError`, `not_implemented()`, `DeviceInfo`. No dependencies on anything else. Everything else depends on these.

2. **`wsdl_loader.rs`** — `EmbeddedWsdlLoader`. Depends only on soap-server's `WsdlLoader` trait and the bundled WSDL bytes. Must exist before `OnvifServer` can build any `SoapService`.

3. **`traits/device.rs`** — `DeviceService` trait + `DeviceServiceHandler`. First service to implement because it is required by every ONVIF device and has the fewest external type dependencies. Also validates the trait-to-handler wiring pattern before replicating across five services.

4. **`server.rs` skeleton** — `OnvifServer` builder, minimal, wires only `DeviceService` initially. Validates end-to-end: embedded WSDL loads, dispatch table builds, axum router serves requests.

5. **`traits/media.rs`** — `MediaService`. Adds `GetProfiles`/`GetStreamUri`; validates multi-service router merging.

6. **`traits/ptz.rs`** — `PTZService`. Largest trait surface; most critical for Frigate. Implement after media so `GetProfiles` profile tokens can be referenced in PTZ ops.

7. **`traits/imaging.rs`** and **`traits/events.rs`** — Lower priority; implement after Frigate compatibility is validated with device + media + ptz.

8. **`discovery.rs`** — Feature-gated; implement last. Does not affect core test path with Frigate.

## Sources

- Direct reading of `soap-server` source: `src/server.rs`, `src/handler.rs`, `src/dispatch.rs`, `src/lib.rs` — HIGH confidence (primary source)
- `docs/DESIGN.md` — Authoritative design intent — HIGH confidence
- ONVIF Core Specification 22.12, Section 5 (service endpoint URL conventions) — HIGH confidence (spec)
- ONVIF Profile S Specification (GetSystemDateAndTime auth exemption requirement) — HIGH confidence (spec)
- python-onvif-zeep server-side patterns (Frigate's `ptz/onvif.py` call sequence) — MEDIUM confidence (inferred from client behavior described in DESIGN.md)

---
*Architecture research for: ONVIF Device Server Rust crate (onvif-server)*
*Researched: 2026-04-05*
