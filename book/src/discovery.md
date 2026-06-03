# WS-Discovery

WS-Discovery enables ONVIF clients to find devices on the local network
automatically without knowing their IP addresses in advance. When a client
sends a multicast `Probe` message, discoverable devices respond with their
service addresses.

## Enabling the feature

WS-Discovery support is behind the optional `discovery` Cargo feature. Add it
to your dependency:

```sh
cargo add onvif-server --features discovery
```

This feature pulls in the `socket2` crate, which is required for the low-level
UDP multicast socket setup.

## Runtime behaviour

When the `discovery` feature is enabled, `OnvifServer::run()` spawns a background
task that:

1. Joins the IPv4 multicast group `239.255.255.250` on port `3702`.
2. Listens for incoming UDP datagrams.
3. Parses each datagram and ignores anything that is not a well-formed WS-Discovery
   `Probe` message (SOAP body first child = `Probe` in namespace
   `http://schemas.xmlsoap.org/ws/2005/04/discovery`).
4. For genuine `Probe` messages, sends a `ProbeMatches` response back to the
   sender's address, embedding the device's service address (XAddr) and its stable
   WS-Discovery EndpointReference UUID.

The XAddr in the `ProbeMatches` response is derived from the `advertised_host` and
`port` configured on the builder:
`http://<advertised_host>:<port>/onvif/device_service`.

## EndpointReference UUID

ONVIF WS-Discovery requires the `EndpointReference/Address` to be a stable
per-device identity across discovery cycles. The configured UUID is fixed for the
lifetime of the server, so every `ProbeMatches` within one process run carries the
same identity.

When not set, the builder assigns a **random UUID-v4 once at build time**: stable
across discovery cycles, but **not across restarts** — a restarted device appears
as a new identity to clients that key on the EndpointReference. Use
`.discovery_uuid(uuid::Uuid)` to supply a fixed UUID derived from a hardware id or
stored config when restart-stable identity matters.

## Deployment hazards

WS-Discovery is multicast UDP and fails quietly in common network topologies.
Before relying on it:

- **`advertised_host` must be client-routable.** The `ProbeMatches` XAddr is
  `http://<advertised_host>:<port>/onvif/device_service`. If `advertised_host` is
  left at `0.0.0.0` (or a container-internal IP), clients discover the device but
  cannot then reach it. Set it to the device's real LAN address.
- **UDP 3702 must be open.** Host firewalls frequently block inbound UDP 3702 and
  the multicast group `239.255.255.250`. Discovery silently returns nothing if it
  is filtered — unlike the SOAP endpoint, there is no connection error to see.
- **Multicast rarely crosses subnets/VLANs.** Probes are link-local; a client on a
  different VLAN or subnet (or across a router without an IGMP/mDNS reflector) will
  not discover the device. Cross-segment clients must be given the XAddr directly.
- **Multiple NICs are ambiguous.** On a multi-homed host the listener joins the
  group, but the address clients should use is whatever you put in
  `advertised_host` — pick the interface clients actually reach.
- **Discovery is optional.** Most integrations (Frigate, Home Assistant) let you
  enter the device URL directly; discovery is a convenience, not a requirement. If
  it misbehaves, configure clients with the explicit
  `http://<host>:<port>/onvif/device_service` URL and move on.

## Low-level helpers

The probe-parsing and probe-response functions are always compiled (no feature
gate) because they are pure XML and useful for testing:

- `onvif_server::discovery_is_probe(msg: &[u8]) -> bool` — returns `true` if
  the bytes are a well-formed WS-Discovery `Probe` message.
- `onvif_server::discovery_build_probe_match(relates_to: &str, xaddr: &str, device_uuid: uuid::Uuid) -> String` — builds a `ProbeMatches` XML response.

Only the UDP multicast listener (`run_discovery`) requires the `discovery` feature.
