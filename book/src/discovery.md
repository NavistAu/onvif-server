# WS-Discovery

WS-Discovery enables ONVIF clients to find devices on the local network
automatically without knowing their IP addresses in advance. When a client
sends a multicast `Probe` message, discoverable devices respond with their
service addresses.

## Enabling the feature

WS-Discovery support is behind the optional `discovery` Cargo feature. Add it
to your dependency:

```toml
[dependencies]
onvif-server = { version = "0.1.0", features = ["discovery"] }
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

ONVIF WS-Discovery requires the `EndpointReference/Address` to be a stable,
per-device identity that does not change across discovery cycles or restarts.
Use `.discovery_uuid(uuid::Uuid)` on the builder to supply a fixed UUID for
your device. When not set, the builder assigns a random UUID-v4 at build time.

## Low-level helpers

The probe-parsing and probe-response functions are always compiled (no feature
gate) because they are pure XML and useful for testing:

- `onvif_server::discovery_is_probe(msg: &[u8]) -> bool` — returns `true` if
  the bytes are a well-formed WS-Discovery `Probe` message.
- `onvif_server::discovery_build_probe_match(relates_to: &str, xaddr: &str, device_uuid: uuid::Uuid) -> String` — builds a `ProbeMatches` XML response.

Only the UDP multicast listener (`run_discovery`) requires the `discovery` feature.
