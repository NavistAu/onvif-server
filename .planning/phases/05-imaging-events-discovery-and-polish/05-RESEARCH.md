# Phase 5: Imaging, Events, Discovery, and Polish - Research

**Researched:** 2026-04-05
**Domain:** ONVIF Imaging Service, Events Service (WS-BaseNotification pull-point), WS-Discovery UDP multicast, ODM smoke test
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Porting operation following best prior art (onvif-rs, python-onvif-zeep, ONVIF spec) — same as Phases 1-4
- Follow established patterns from the research
- Hand-written types (not codegen) — same approach throughout
- ServiceHandler dispatch pattern (extract_local_name + match) proven in Phases 2-4
- Builder already accepts `.imaging_service(impl)` and `.event_service(impl)` — needs wiring in run()
- Router::merge() pattern for multi-service wiring proven
- Token constants all defined

### Claude's Discretion
- ImagingServiceHandler implementation (minimal — single GetImagingSettings operation delegates to trait)
- EventServiceHandler with in-memory subscription state (HashMap<subscription_id, subscription_info>)
- PullMessages queue implementation (Vec<EventNotification> per subscription, consumer pushes events)
- WS-Discovery UDP multicast implementation behind `discovery` feature flag using socket2
- ODM smoke test structure (TEST-03) — what specific operations to test
- Whether Events service needs its own WSDL or shares with Device service
- All technical decisions follow research and DESIGN.md

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| IMG-01 | GetImagingSettings with a video source token returns imaging settings from the consumer's trait implementation | ImagingSettings20 XSD type confirmed in onvif.xsd line 5959 — all fields optional floats; timg namespace is `http://www.onvif.org/ver20/imaging/wsdl`; VideoSourceToken extracted from request body; response element is `timg:ImagingSettings` of type `tt:ImagingSettings20` |
| EVT-01 | GetEventProperties returns supported event topics from the consumer's trait implementation | GetEventPropertiesResponse per events.wsdl lines 219-270: TopicNamespaceLocation, wsnt:FixedTopicSet, wstop:TopicSet, wsnt:TopicExpressionDialect, MessageContentFilterDialect — all mandatory; tev namespace is `http://www.onvif.org/ver10/events/wsdl` |
| EVT-02 | CreatePullPointSubscription returns a subscription reference for polling events | SubscriptionReference contains wsa5:Address (URL to pull endpoint); wsnt:CurrentTime + wsnt:TerminationTime required in response; subscription ID embedded in address URL using uuid crate already present |
| EVT-03 | PullMessages on a subscription returns queued event notifications | Response requires CurrentTime + TerminationTime + zero or more wsnt:NotificationMessage elements; handler-internal state (no trait delegation needed per CONTEXT) |
| EVT-04 | Unsubscribe terminates an event subscription | Unsubscribe reads SubscriptionId from request, removes from in-memory HashMap, returns empty response; Unsubscribe is dispatched to the Events handler via the same path |
| DISC-01 | When `discovery` feature flag enabled, server responds to WS-Discovery Probe messages on UDP multicast 239.255.255.250:3702 | tokio::spawn in run() when cfg!(feature="discovery"); socket2 creates UDP socket, joins multicast group, receives Probe, sends ProbeMatch as unicast reply to sender addr |
| DISC-02 | WS-Discovery ProbeMatch responses include the device's XAddrs and scopes | ProbeMatch XML format confirmed from agsh/onvif mockup: Types="dn:NetworkVideoTransmitter tds:Device", Scopes per device, XAddrs=device_service URL; wsdd namespace `http://schemas.xmlsoap.org/ws/2005/04/discovery`, wsa namespace `http://schemas.xmlsoap.org/ws/2004/08/addressing` |
| TEST-03 | ONVIF Device Manager smoke test validates basic device discovery and info retrieval | Implemented as in-process integration test (same pattern as frigate_compat.rs) calling GetDeviceInformation, GetCapabilities, GetServices on DeviceServiceHandler — ODM's first-connect sequence |
</phase_requirements>

## Summary

Phase 5 completes the v1 ONVIF service surface by implementing four independent concerns: the Imaging service handler (single operation, thin trait delegation), the Events service handler (three operations with in-memory subscription state), WS-Discovery (feature-gated UDP task), and an ODM smoke test.

All four service handler additions follow the pattern proven in Phases 2-4: one handler struct implementing `SoapHandler`, `extract_local_name` dispatch, format-string XML responses, and trait delegation for consumer-facing operations. No new dependencies are required for the service handlers — the dependency set (`quick-xml`, `bytes`, `async-trait`, `uuid`, `chrono`, `socket2`) is already present.

The Events service introduces the only new runtime state in this codebase: a `HashMap<String, SubscriptionState>` inside `EventServiceHandler` behind an `Arc<Mutex<_>>`. The subscription ID is generated via `uuid::Uuid::new_v4()`, embedded in the pull endpoint URL, and extracted from `PullMessages`/`Unsubscribe` request bodies. PullMessages returns an empty message list (the consumer pushes events asynchronously — a queue mechanism is discretionary). WS-Discovery is a separate `tokio::task` spawned in `run()` when `#[cfg(feature = "discovery")]`; it uses `socket2` to bind a UDP socket, join the `239.255.255.250` multicast group, receive `Probe` datagrams, and reply with a `ProbeMatch` to the sender's unicast address.

**Primary recommendation:** Implement in wave order — Imaging first (simplest), then Events (new state pattern), then Discovery (separate task), then ODM smoke test (integration-level, exercising the full five-service surface).

---

## Standard Stack

### Core (no new dependencies required)

| Library | Version | Purpose | Already Present |
|---------|---------|---------|----------------|
| `quick-xml` | 0.39 | Extract VideoSourceToken, subscription IDs from request bodies | Yes |
| `bytes` | 1 | `Bytes` in/out for `SoapHandler::handle` | Yes |
| `async-trait` | 0.1 | `#[async_trait]` on `ImagingService` and `EventService` traits | Yes |
| `soap-server` | path dep | `SoapHandler`, `SoapFault`, `ServerBuilder` | Yes |
| `axum` | 0.8 | `Router::merge()` for 4th and 5th service routing | Yes |
| `uuid` | 1 (v4 feature) | Generate subscription IDs via `Uuid::new_v4()` | Yes |
| `chrono` | 0.4 | ISO 8601 timestamps for CurrentTime/TerminationTime in Events responses | Yes |
| `socket2` | 0.5 (optional) | UDP multicast socket join for WS-Discovery | Yes (optional) |
| `tokio` | 1 | `tokio::spawn` for Discovery task; `sync` feature for Mutex | Yes |

**No additions to Cargo.toml required.** All dependencies were added in Phase 1-4.

### Installation
```bash
# No new packages. Existing Cargo.toml is complete.
# To test discovery feature:
cargo test --features discovery
```

## Architecture Patterns

### Recommended Project Structure Changes

```
src/
├── service/
│   ├── mod.rs          # add: pub mod imaging; pub mod events;
│   ├── imaging.rs      # NEW: ImagingServiceHandler (single operation)
│   └── events.rs       # NEW: EventServiceHandler (subscription state)
├── generated/
│   └── types.rs        # add: ImagingSettings struct
├── traits/
│   ├── imaging.rs      # update: typed signature for get_imaging_settings
│   └── events.rs       # update: typed signatures (get_event_properties only)
├── server.rs           # update: wire Imaging+Events in run(); spawn Discovery task
└── lib.rs              # update: pub use ImagingServiceHandler, EventServiceHandler
examples/
└── virtual_ptz.rs      # update: add ImagingService + EventService impls
tests/
├── imaging_service.rs  # NEW: ImagingServiceHandler unit tests
├── events_service.rs   # NEW: EventServiceHandler unit tests (subscription lifecycle)
└── odm_smoke.rs        # NEW: TEST-03 ODM call sequence integration test
```

### Pattern 1: ImagingServiceHandler — Thin Trait Delegation

`GetImagingSettings` is the only IMG-01 operation. The handler extracts `VideoSourceToken` from the request body, calls the trait, and serializes the returned `ImagingSettings` struct to XML.

**What:** Single-operation handler; all other Imaging operations return `ActionNotSupported`.
**When to use:** Following the same approach as MediaServiceHandler with static handler-internal responses for discovery operations.

```rust
// src/service/imaging.rs
use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};
use crate::error::OnvifError;
use crate::traits::ImagingService;
use crate::generated::ImagingSettings;

pub struct ImagingServiceHandler {
    pub(crate) svc: Arc<dyn ImagingService>,
}

#[async_trait]
impl SoapHandler for ImagingServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetImagingSettings" => self.handle_get_imaging_settings(&body).await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}

// Response namespace: timg = http://www.onvif.org/ver20/imaging/wsdl
// Response namespace: tt   = http://www.onvif.org/ver10/schema
// ImagingSettings children are all optional — only emit what the consumer returns
```

### Pattern 2: EventServiceHandler — In-Memory Subscription State

The Events service manages pull-point subscriptions via an `Arc<Mutex<HashMap<String, SubscriptionInfo>>>`. The subscription ID is a UUID v4 embedded in the pull endpoint URL. `PullMessages` and `Unsubscribe` extract the subscription ID from the request body.

**What:** Three operations (CreatePullPointSubscription, PullMessages, Unsubscribe) plus GetEventProperties. Subscription state lives in the handler struct, not the trait.

```rust
// src/service/events.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

pub struct SubscriptionInfo {
    pub termination_time: chrono::DateTime<Utc>,
    // future: queue of pending notifications
}

pub struct EventServiceHandler {
    pub(crate) svc: Arc<dyn EventService>,
    pub(crate) xaddr: String,
    subscriptions: Arc<Mutex<HashMap<String, SubscriptionInfo>>>,
}

// CreatePullPointSubscription:
//   1. sub_id = Uuid::new_v4().to_string()
//   2. insert into subscriptions map
//   3. pull_url = format!("{}/subscriptions/{}", xaddr, sub_id)
//   4. return SubscriptionReference with wsa5:Address = pull_url
//      + wsnt:CurrentTime + wsnt:TerminationTime

// PullMessages:
//   1. extract sub_id from request (SubscriptionId or from WS-Addressing To header)
//   2. look up in subscriptions
//   3. return empty message list (CurrentTime + TerminationTime + no NotificationMessage)

// Unsubscribe:
//   1. extract sub_id from request
//   2. remove from subscriptions
//   3. return empty UnsubscribeResponse
```

**Key design note:** `PullMessages` is sent TO the subscription endpoint URL (not the events service URL). In practice, for this implementation, the Events service handler serves all paths under its registered prefix. The subscription ID in the URL is carried in the SOAP body or WS-Addressing `To` header — the simplest approach is to also include a `<SubscriptionId>` element in the response and parse it back from `PullMessages`/`Unsubscribe` request bodies.

### Pattern 3: WS-Discovery UDP Task

Spawned as a background `tokio::task` in `OnvifServer::run()` when `#[cfg(feature = "discovery")]`. It runs an infinite receive loop, parsing each incoming datagram for a `Probe` message and sending a `ProbeMatch` back to the sender.

```rust
// In server.rs, inside run(), conditional on feature:
#[cfg(feature = "discovery")]
{
    let disc_xaddr = format!("http://0.0.0.0:{}/onvif/device_service", self.port);
    tokio::spawn(async move {
        if let Err(e) = run_discovery(disc_xaddr).await {
            eprintln!("[discovery] task exited: {e}");
        }
    });
}

// Separate function (or module src/discovery.rs):
#[cfg(feature = "discovery")]
async fn run_discovery(xaddr: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use socket2::{Socket, Domain, Type, Protocol};
    use std::net::{SocketAddr, Ipv4Addr};

    let multicast_addr: Ipv4Addr = "239.255.255.250".parse().unwrap();
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddr::from((multicast_addr, 3702)).into())?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_nonblocking(true)?;

    let udp: std::net::UdpSocket = socket.into();
    // Wrap in tokio::net::UdpSocket for async recv_from
    let udp = tokio::net::UdpSocket::from_std(udp)?;

    let mut buf = vec![0u8; 65535];
    loop {
        let (len, src) = udp.recv_from(&mut buf).await?;
        let msg = &buf[..len];
        if is_probe(msg) {
            let msg_id = extract_message_id(msg).unwrap_or_else(|| Uuid::new_v4().to_string());
            let reply = build_probe_match(&msg_id, &xaddr);
            udp.send_to(reply.as_bytes(), src).await?;
        }
    }
}
```

**Binding note:** On Linux/macOS, bind to the multicast address (239.255.255.250:3702). On Windows, bind to 0.0.0.0:3702 instead. The `is_probe` check looks for `wsdd:Probe` in the datagram body — a string search on the raw bytes suffices (the XML body contains the element name).

### Pattern 4: ODM Smoke Test Structure

TEST-03 uses the same in-process handler-call pattern as `frigate_compat.rs`. It exercises the call sequence ODM uses when it first connects: GetCapabilities → GetDeviceInformation → GetServices. No HTTP server needed; direct `handler.handle(body).await` calls only.

```rust
// tests/odm_smoke.rs — mirrors frigate_compat.rs structure
// Sequence:
//   1. Device::GetCapabilities → asserts XAddrs present for media/ptz/imaging/events
//   2. Device::GetDeviceInformation → asserts manufacturer/model present
//   3. Device::GetServices → asserts all registered services listed
//   4. Imaging::GetImagingSettings → asserts ImagingSettings element present
//   5. Events::CreatePullPointSubscription → asserts SubscriptionReference present
//   6. Events::PullMessages → asserts CurrentTime + TerminationTime present
//   7. Events::Unsubscribe → asserts Ok (no fault)
```

### Anti-Patterns to Avoid

- **Blocking the tokio runtime in the discovery task:** Use `tokio::net::UdpSocket::from_std()` — never block an async task with synchronous socket I/O.
- **Storing Arc<dyn EventService> for state the trait doesn't need:** `GetEventProperties` delegates to the trait; subscription management is handler-internal state only.
- **Forgetting to wire Imaging and Events in run():** Builder accepts them but `run()` currently only wires Device/Media/PTZ. Both need new `soap_server::ServerBuilder` blocks and `.merge()` calls.
- **Publishing Imaging service xaddr in GetCapabilities/GetServices:** DeviceServiceHandler must be updated to include Imaging and Events XAddrs in its static responses when those services are registered.
- **Missing username/password clone for new service blocks:** Each `soap_server::ServerBuilder::auth()` closure requires its own clone of `username`/`password` (see current `username3`/`password3` pattern in server.rs).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UUID generation for subscription IDs | Custom ID counter | `uuid::Uuid::new_v4()` | Already a dependency; collision-free; ODM expects UUID format |
| ISO 8601 timestamps | Custom date formatter | `chrono::Utc::now().to_rfc3339()` | Already a dependency; handles timezone correctly |
| Multicast socket setup | `std::net::UdpSocket` direct | `socket2::Socket` + `join_multicast_v4` | std::net doesn't expose `IP_ADD_MEMBERSHIP` socket option needed for multicast join |
| Probe XML parsing | Full XML parse | String `contains("Probe")` check | WS-Discovery Probe body is small; element name presence is sufficient to distinguish from non-Probe datagrams on port 3702 |

**Key insight:** The subscription state map (`HashMap<String, SubscriptionInfo>`) is a 10-line implementation, not a library problem. The real complexity is the XML response structure (WS-Addressing, WSNT namespaces) — the templates must exactly match what ODM parses.

## Common Pitfalls

### Pitfall 1: Imaging Service xaddr — GetCapabilities Must Advertise It

**What goes wrong:** ImagingServiceHandler is wired but GetCapabilities (in DeviceServiceHandler) still returns only Device/Media/PTZ XAddrs. ODM calls GetCapabilities first and uses the returned XAddrs to build service stubs — if Imaging/Events aren't listed, ODM won't call them.

**Why it happens:** DeviceServiceHandler builds its GetCapabilities XML from static constants. Phase 5 adds two more services, but the Device handler has no reference to the Imaging/Events handlers.

**How to avoid:** DeviceServiceHandler must accept a `services: Vec<(String, String)>` or similar (namespace → xaddr pairs) so `run()` can inject Imaging and Events XAddrs. The existing pattern in `DeviceServiceHandler` uses a single `xaddr` field — this needs extending to a service registry or a dedicated capabilities struct.

**Warning signs:** ODM connects but shows only Device/Media/PTZ in its service list.

### Pitfall 2: Events Service — PullMessages Sent to the Subscription URL, Not the Events Service URL

**What goes wrong:** ODM sends `PullMessages` to the pull-point URL returned in `CreatePullPointSubscriptionResponse.SubscriptionReference.Address` (e.g., `http://host:8080/onvif/events/subscriptions/{sub_id}`), NOT to `/onvif/events_service`. If the router doesn't handle this path, ODM gets 404.

**Why it happens:** ONVIF pull-point subscriptions are defined as separate endpoint addresses in the WS-BaseNotification spec. ONVIF clients that strictly follow the spec POST PullMessages to the subscription URL.

**How to avoid:** Register the events service at a path prefix that will match both `/onvif/events_service` and `/onvif/events_service/subscriptions/*`. Alternatively, use a single events path `/onvif/events_service` and embed the subscription ID in the SOAP body instead of the URL, then ignore the URL distinction. The simplest approach: treat all requests to the events SOAP handler as subscription-aware — extract subscription ID from body regardless of path.

**Practical shortcut:** soap-server mounts on a fixed path. Register events at `/onvif/events_service` and put the subscription ID in the `<tev:SubscriptionReference><wsa5:Address>` pointing back to the same path with a query param or just `http://host/onvif/events_service` (ODM will POST PullMessages there, where the handler reads the subscription ID from a `<SubscriptionId>` element included in the response and echoed back by the client).

**Warning signs:** ODM gets 404 on PullMessages after successful CreatePullPointSubscription.

### Pitfall 3: WS-Discovery — Bind Address Platform Difference

**What goes wrong:** On Windows, binding a socket to `239.255.255.250:3702` fails; you must bind to `0.0.0.0:3702`. On Unix, both work but binding to the multicast address is preferred to avoid receiving all UDP traffic on port 3702.

**Why it happens:** `IP_ADD_MEMBERSHIP` and bind semantics differ between platforms.

**How to avoid:** Use `#[cfg(unix)]` / `#[cfg(windows)]` to select bind address. The socket2 crate's `Socket::bind()` is platform-aware, but the address itself must be chosen per-platform.

**Warning signs:** `socket.bind()` returns `EADDRNOTAVAIL` on Windows when bound to multicast address.

### Pitfall 4: Events WSDL — WS-BaseNotification External Import

**What goes wrong:** `events.wsdl` imports `http://docs.oasis-open.org/wsn/bw-2.wsdl` and `http://docs.oasis-open.org/wsrf/rw-2.wsdl` — remote URLs that `EmbeddedWsdlLoader` can't resolve. If `soap-server::ServerBuilder` attempts to resolve these imports, the build will fail.

**Why it happens:** The events WSDL has external dependencies on OASIS standards documents. The existing `EmbeddedWsdlLoader` already handles the XSD imports (b-2.xsd stub in `wsdl/wsn-b2.xsd`) but not the WSDL imports.

**How to avoid:** Check whether `soap-server::ServerBuilder` resolves WSDL imports recursively (not just XSD imports). If it does, add stub WSDL bytes for `bw-2.wsdl` and `rw-2.wsdl` to `EmbeddedWsdlLoader`. If not, `events.wsdl` loads without issue. The existing behavior (prior phases successfully mounted `media.wsdl` and `ptz.wsdl` which have similar XSD import chains) suggests the loader handles this gracefully already.

**Warning signs:** `ServerBuilder::build()` returns `MalformedXml` for events.wsdl during `run()`.

### Pitfall 5: ImagingSettings Type — Consumer Returns Struct, Handler Serializes Selectively

**What goes wrong:** `ImagingSettings20` has 10 optional fields (Brightness, Contrast, Sharpness, etc.). Serializing all fields unconditionally produces XML with empty/zero values that ODM may reject or misinterpret.

**Why it happens:** XML Schema `minOccurs="0"` fields should be omitted entirely when absent, not emitted as empty elements.

**How to avoid:** Define `ImagingSettings` in `generated/types.rs` using `Option<f32>` for all scalar fields. The handler serializes only `Some` fields:
```rust
// Only emit fields that are Some
if let Some(v) = settings.brightness {
    xml.push_str(&format!("<tt:Brightness>{v}</tt:Brightness>"));
}
```

## Code Examples

Verified patterns from official ONVIF specifications and the existing codebase:

### GetImagingSettingsResponse XML
```xml
<!-- Source: imaging.wsdl GetImagingSettingsResponse element; onvif.xsd ImagingSettings20 type -->
<timg:GetImagingSettingsResponse
    xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"
    xmlns:tt="http://www.onvif.org/ver10/schema">
  <timg:ImagingSettings>
    <tt:Brightness>50</tt:Brightness>
    <tt:Contrast>50</tt:Contrast>
    <tt:Sharpness>50</tt:Sharpness>
  </timg:ImagingSettings>
</timg:GetImagingSettingsResponse>
```

### CreatePullPointSubscriptionResponse XML
```xml
<!-- Source: events.wsdl CreatePullPointSubscriptionResponse; agsh/onvif mockup verified -->
<!-- tev = http://www.onvif.org/ver10/events/wsdl -->
<!-- wsa5 = http://www.w3.org/2005/08/addressing -->
<!-- wsnt = http://docs.oasis-open.org/wsn/b-2 -->
<tev:CreatePullPointSubscriptionResponse
    xmlns:tev="http://www.onvif.org/ver10/events/wsdl"
    xmlns:wsa5="http://www.w3.org/2005/08/addressing"
    xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2">
  <tev:SubscriptionReference>
    <wsa5:Address>http://192.168.1.10:8080/onvif/events_service</wsa5:Address>
    <wsa5:ReferenceParameters>
      <tev:SubscriptionId>{sub_uuid}</tev:SubscriptionId>
    </wsa5:ReferenceParameters>
  </tev:SubscriptionReference>
  <wsnt:CurrentTime>{rfc3339}</wsnt:CurrentTime>
  <wsnt:TerminationTime>{rfc3339 + 60s}</wsnt:TerminationTime>
</tev:CreatePullPointSubscriptionResponse>
```

**Note:** Including the subscription ID in `wsa5:ReferenceParameters` under the same endpoint URL is the pragmatic approach. It avoids the path-routing problem (Pitfall 2) and matches how lightweight ONVIF device implementations handle pull-points.

### PullMessagesResponse XML (empty queue)
```xml
<!-- Source: events.wsdl PullMessagesResponse; tev namespace inline -->
<tev:PullMessagesResponse
    xmlns:tev="http://www.onvif.org/ver10/events/wsdl"
    xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2">
  <tev:CurrentTime>{rfc3339}</tev:CurrentTime>
  <tev:TerminationTime>{rfc3339 + 60s}</tev:TerminationTime>
  <!-- No NotificationMessage elements = empty queue, valid per spec -->
</tev:PullMessagesResponse>
```

### GetEventPropertiesResponse XML (minimal)
```xml
<!-- Source: events.wsdl GetEventPropertiesResponse element (lines 219-270) -->
<tev:GetEventPropertiesResponse
    xmlns:tev="http://www.onvif.org/ver10/events/wsdl"
    xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"
    xmlns:wstop="http://docs.oasis-open.org/wsn/t-1">
  <tev:TopicNamespaceLocation>http://www.onvif.org/onvif/ver10/topics/topicns.xml</tev:TopicNamespaceLocation>
  <wsnt:FixedTopicSet>true</wsnt:FixedTopicSet>
  <wstop:TopicSet/>
  <wsnt:TopicExpressionDialect>http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete</wsnt:TopicExpressionDialect>
  <wsnt:TopicExpressionDialect>http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet</wsnt:TopicExpressionDialect>
  <tev:MessageContentFilterDialect>http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter</tev:MessageContentFilterDialect>
</tev:GetEventPropertiesResponse>
```

**Note on trait delegation for GetEventProperties:** EVT-01 says it must return topics "from the consumer's trait implementation." The simplest approach is to delegate the entire response XML to the trait, returning `String` — this gives consumers full control over the topic set they advertise without requiring a complex `TopicSet` Rust type.

### WS-Discovery ProbeMatch XML
```xml
<!-- Source: agsh/onvif Probe.xml mockup; EdgeX WS-Discovery docs verified -->
<!-- Sent as unicast UDP reply to the Probe sender address -->
<s:Envelope
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:wsdd="http://schemas.xmlsoap.org/ws/2005/04/discovery"
    xmlns:dn="http://www.onvif.org/ver10/network/wsdl"
    xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Header>
    <wsa:MessageID>urn:uuid:{new_uuid}</wsa:MessageID>
    <wsa:RelatesTo>{probe_message_id}</wsa:RelatesTo>
    <wsa:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</wsa:To>
    <wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches</wsa:Action>
  </s:Header>
  <s:Body>
    <wsdd:ProbeMatches>
      <wsdd:ProbeMatch>
        <wsa:EndpointReference>
          <wsa:Address>urn:uuid:{device_uuid}</wsa:Address>
        </wsa:EndpointReference>
        <wsdd:Types>dn:NetworkVideoTransmitter tds:Device</wsdd:Types>
        <wsdd:Scopes>onvif://www.onvif.org/Profile/Streaming onvif://www.onvif.org/type/video_encoder</wsdd:Scopes>
        <wsdd:XAddrs>{device_service_xaddr}</wsdd:XAddrs>
        <wsdd:MetadataVersion>1</wsdd:MetadataVersion>
      </wsdd:ProbeMatch>
    </wsdd:ProbeMatches>
  </s:Body>
</s:Envelope>
```

**Key fields:**
- `wsa:RelatesTo` must echo the `wsa:MessageID` from the Probe.
- `wsdd:XAddrs` is the device service HTTP URL.
- ONVIF 2004/08 WS-Addressing namespace is used (NOT 2005/08).

### socket2 Multicast Join Pattern
```rust
// Source: pusateri gist (verified); socket2 0.5 docs
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr};

let multicast_addr: Ipv4Addr = "239.255.255.250".parse().unwrap();
let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
socket.set_reuse_address(true)?;
#[cfg(unix)]
socket.set_reuse_port(true)?;
// Unix: bind to multicast addr; Windows: bind to 0.0.0.0
#[cfg(unix)]
socket.bind(&SocketAddr::from((multicast_addr, 3702u16)).into())?;
#[cfg(windows)]
socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3702u16)).into())?;
socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;
socket.set_nonblocking(true)?;
let udp: std::net::UdpSocket = socket.into();
let udp = tokio::net::UdpSocket::from_std(udp)?;
```

### ImagingSettings Type Addition to generated/types.rs
```rust
// Source: onvif.xsd ImagingSettings20 type (line 5959); all fields minOccurs="0"
/// Returned by ImagingService::get_imaging_settings().
/// Only non-None fields are serialized to XML by ImagingServiceHandler.
#[derive(Debug, Clone, Default)]
pub struct ImagingSettings {
    pub brightness: Option<f32>,
    pub color_saturation: Option<f32>,
    pub contrast: Option<f32>,
    pub sharpness: Option<f32>,
    // Extend with BacklightCompensation, Exposure, Focus, etc. as needed
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| WS-Discovery via dedicated discovery crate | Use socket2 directly with tokio async wrapping | N/A — socket2 is already a dependency | No additional dep; full control of socket lifecycle |
| Events via push (WS-BaseNotification subscribe) | Pull-point pattern (CreatePullPointSubscription + PullMessages) | ONVIF Profile S, always | ODM uses pull-point; push subscriptions not required for v1 |
| Full ONVIF ImagingSettings codegen | Hand-written minimal ImagingSettings struct | Phase 1 decision (stays) | Only expose what consumers can meaningfully use |

**Deprecated/outdated:**
- WS-Discovery 2005/04 addressing vs 2004/08: The ProbeMatch XML uses the OLDER `http://schemas.xmlsoap.org/ws/2004/08/addressing` namespace for the `wsa:` prefix — NOT `http://www.w3.org/2005/08/addressing`. This is confirmed by the agsh/onvif mockup and is the namespace that real ONVIF clients (including ODM) expect in WS-Discovery responses.

## Open Questions

1. **DeviceServiceHandler capability advertisement for Imaging/Events**
   - What we know: DeviceServiceHandler.xaddr is a single string; GetCapabilities/GetServices build XML from it
   - What's unclear: Does DeviceServiceHandler need a full redesign to accept multiple service XAddrs, or just additional optional fields?
   - Recommendation: Add `imaging_xaddr: Option<String>` and `events_xaddr: Option<String>` fields to `DeviceServiceHandler`, set from `run()` when those services are registered. This is the minimal change consistent with the existing pattern.

2. **Events WSDL import — bw-2.wsdl and rw-2.wsdl**
   - What we know: `events.wsdl` has two `wsdl:import` statements pointing to remote OASIS WSDL documents; `EmbeddedWsdlLoader` handles XSD but not WSDL imports
   - What's unclear: Whether `soap-server::ServerBuilder` tries to resolve `wsdl:import` elements
   - Recommendation: Attempt to register events service in a test build; if `ServerBuilder::build()` fails, add minimal stub WSDL bytes to `EmbeddedWsdlLoader` for `bw-2.wsdl` and `rw-2.wsdl`.

3. **Unsubscribe request routing — same path as Events service?**
   - What we know: ONVIF clients send Unsubscribe to the subscription endpoint URL, which in WS-BaseNotification is a separate endpoint from the notification producer
   - What's unclear: Whether ODM sends Unsubscribe to the same events service endpoint or to a different URL
   - Recommendation: Register Unsubscribe handling on the same `/onvif/events_service` endpoint. Extract subscription ID from body. ODM in practice sends to the events service URL.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + tokio-test (already configured) |
| Config file | none — Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test` |
| Full suite command | `cargo test --features discovery` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IMG-01 | GetImagingSettings delegates to trait and returns ImagingSettings XML | unit | `cargo test imaging_get_imaging_settings` | ❌ Wave 0 |
| EVT-01 | GetEventProperties returns topic namespace XML | unit | `cargo test events_get_event_properties` | ❌ Wave 0 |
| EVT-02 | CreatePullPointSubscription returns SubscriptionReference with UUID address | unit | `cargo test events_create_pull_point_subscription` | ❌ Wave 0 |
| EVT-03 | PullMessages returns CurrentTime + TerminationTime with no panic | unit | `cargo test events_pull_messages` | ❌ Wave 0 |
| EVT-04 | Unsubscribe removes subscription and returns Ok | unit | `cargo test events_unsubscribe` | ❌ Wave 0 |
| EVT-02+EVT-03+EVT-04 | Full subscription lifecycle without panic | unit | `cargo test events_subscription_lifecycle` | ❌ Wave 0 |
| DISC-01 | Discovery task starts without panic (feature flag) | unit | `cargo test --features discovery discovery_starts` | ❌ Wave 0 |
| DISC-02 | ProbeMatch response XML contains XAddrs and Types | unit | `cargo test --features discovery discovery_probe_match` | ❌ Wave 0 |
| TEST-03 | ODM call sequence: GetCapabilities→GetDeviceInformation→GetServices→GetImagingSettings | integration | `cargo test odm_smoke` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test --features discovery`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/imaging_service.rs` — covers IMG-01
- [ ] `tests/events_service.rs` — covers EVT-01 through EVT-04
- [ ] `tests/odm_smoke.rs` — covers TEST-03
- [ ] `src/service/imaging.rs` — ImagingServiceHandler stub
- [ ] `src/service/events.rs` — EventServiceHandler stub

## Sources

### Primary (HIGH confidence)
- `wsdl/imaging.wsdl` (bundled, lines 58-83) — GetImagingSettings request/response element structure, VideoSourceToken, ImagingSettings20 type reference
- `wsdl/events.wsdl` (bundled, lines 87-270) — CreatePullPointSubscription, PullMessages, GetEventProperties element structures; SubscriptionReference type; tev namespace
- `wsdl/onvif.xsd` (bundled, lines 5959-6008) — ImagingSettings20 XSD definition, all fields optional floats
- `src/service/ptz.rs` — authoritative in-project example of the dispatch pattern, extract_local_name, format-string XML responses
- `src/server.rs` — run() wiring pattern, Router::merge(), username/password cloning per service

### Secondary (MEDIUM confidence)
- [agsh/onvif ProbeMatch mockup](https://github.com/agsh/onvif/blob/master/test/serverMockup/Probe.xml) — ProbeMatch XML structure with namespace declarations
- [agsh/onvif CreatePullPointSubscription mockup](https://github.com/agsh/onvif/blob/master/test/serverMockup/events.CreatePullPointSubscription.xml) — CreatePullPointSubscription response XML
- [pusateri multicast gist](https://gist.github.com/pusateri/df98511b88e9000f388d344a1f3db9e7) — socket2 multicast join pattern, platform-specific bind
- [EdgeX WS-Discovery docs](https://docs.edgexfoundry.org/4.0/microservices/device/services/device-onvif-camera/supplementary-info/ws-discovery/) — Probe message header fields, multicast address/port confirmation

### Tertiary (LOW confidence)
- WebSearch results for ImagingSettings20 namespace structure — corroborated by bundled WSDL/XSD, elevated to HIGH

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps already present in Cargo.toml; no new library decisions required
- Architecture: HIGH — four independent concerns, each following proven in-project patterns; only Events introduces new state pattern
- Pitfalls: HIGH — derived from reading existing server.rs, events.wsdl, and direct experience in Phases 2-4; platform socket pitfall from WS-Discovery spec docs

**Research date:** 2026-04-05
**Valid until:** 2026-07-05 (stable domain — ONVIF specs and socket2 API are not fast-moving)
