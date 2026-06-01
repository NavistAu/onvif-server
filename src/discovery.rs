//! WS-Discovery support. The probe-detection and response-building functions are
//! always compiled (pure XML, testable without sockets); the UDP multicast listener
//! (`run_discovery`) is only compiled with the `discovery` feature.

/// WS-Discovery namespace for Probe messages.
const WSD_NS: &str = "http://schemas.xmlsoap.org/ws/2005/04/discovery";

#[cfg(feature = "discovery")]
pub async fn run_discovery(xaddr: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_discovery_with_uuid(xaddr, uuid::Uuid::new_v4()).await
}

/// Inner implementation that accepts an explicit device UUID so tests can assert
/// stable identity across multiple probes.
#[cfg(feature = "discovery")]
pub(crate) async fn run_discovery_with_uuid(
    xaddr: String,
    device_uuid: uuid::Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use socket2::{Domain, Protocol, Socket, Type};
    use std::net::{Ipv4Addr, SocketAddr};

    let multicast_addr: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
    let bind_addr: Ipv4Addr = {
        #[cfg(unix)]
        {
            multicast_addr
        }
        #[cfg(not(unix))]
        {
            Ipv4Addr::UNSPECIFIED
        }
    };

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddr::from((bind_addr, 3702u16)).into())?;
    socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_nonblocking(true)?;

    let std_udp: std::net::UdpSocket = socket.into();
    let udp = tokio::net::UdpSocket::from_std(std_udp)?;

    let mut buf = vec![0u8; 65535];
    loop {
        let (len, src) = udp.recv_from(&mut buf).await?;
        let msg = &buf[..len];

        // Only respond to genuine WS-Discovery Probe messages (namespace-qualified).
        if !is_probe_message(msg) {
            continue;
        }

        let msg_id = extract_message_id(msg).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let reply = build_probe_match(&msg_id, &xaddr, device_uuid);
        let _ = udp.send_to(reply.as_bytes(), src).await;
    }
}

/// Returns `true` only when the UDP payload is a well-formed SOAP message whose
/// first `Body` child element has local name `Probe` in the WS-Discovery namespace.
/// Any other payload (including packets that merely contain the byte sequence "Probe")
/// returns `false`.
///
/// Exposed as `pub` so the library's public `discovery_is_probe` re-export can
/// be used in integration tests without enabling the `discovery` feature.
pub fn is_probe_message(msg: &[u8]) -> bool {
    use quick_xml::events::Event;
    use quick_xml::NsReader;

    let Ok(text) = std::str::from_utf8(msg) else {
        return false;
    };

    let mut reader = NsReader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut in_body = false;

    loop {
        match reader.read_resolved_event() {
            Ok((ns, Event::Start(e))) | Ok((ns, Event::Empty(e))) => {
                let local = e.local_name();
                let local_str = match std::str::from_utf8(local.as_ref()) {
                    Ok(s) => s,
                    Err(_) => return false,
                };

                if local_str == "Body" {
                    in_body = true;
                    continue;
                }

                if in_body && local_str == "Probe" {
                    // Must be in the WS-Discovery namespace.
                    if let quick_xml::name::ResolveResult::Bound(n) = ns {
                        if n.as_ref() == WSD_NS.as_bytes() {
                            return true;
                        }
                    }
                    // Wrong namespace — not a genuine Probe.
                    return false;
                }
            }
            Ok((_, Event::End(e))) => {
                let local = e.local_name();
                if let Ok("Body") = std::str::from_utf8(local.as_ref()) {
                    // Exiting Body without finding a Probe.
                    return false;
                }
            }
            Ok((_, Event::Eof)) => return false,
            Err(_) => return false,
            _ => {}
        }
    }
}

#[cfg(feature = "discovery")]
fn extract_message_id(msg: &[u8]) -> Option<String> {
    // Find <a:MessageID> or <wsa:MessageID> text content via simple byte search
    // Returns the UUID string between the tags, or None if not found
    let text = std::str::from_utf8(msg).ok()?;
    let start = text.find("MessageID>")?.checked_add("MessageID>".len())?;
    let end = text[start..].find('<')?.checked_add(start)?;
    Some(text[start..end].trim().to_string())
}

pub fn build_probe_match(relates_to: &str, xaddr: &str, device_uuid: uuid::Uuid) -> String {
    use soap_server::escape_text;
    let msg_id = uuid::Uuid::new_v4();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope
    xmlns:s="http://www.w3.org/2003/05/soap-envelope"
    xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
    xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"
    xmlns:dn="http://www.onvif.org/ver10/network/wsdl"
    xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Header>
    <a:MessageID>urn:uuid:{msg_id}</a:MessageID>
    <a:RelatesTo>{relates_to}</a:RelatesTo>
    <a:To>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:To>
    <a:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches</a:Action>
  </s:Header>
  <s:Body>
    <d:ProbeMatches>
      <d:ProbeMatch>
        <a:EndpointReference><a:Address>urn:uuid:{device_uuid}</a:Address></a:EndpointReference>
        <d:Types>dn:NetworkVideoTransmitter tds:Device</d:Types>
        <d:Scopes>onvif://www.onvif.org/type/NetworkVideoTransmitter</d:Scopes>
        <d:XAddrs>{xaddr}</d:XAddrs>
        <d:MetadataVersion>1</d:MetadataVersion>
      </d:ProbeMatch>
    </d:ProbeMatches>
  </s:Body>
</s:Envelope>"#,
        msg_id = msg_id,
        relates_to = escape_text(relates_to),
        device_uuid = device_uuid,
        xaddr = escape_text(xaddr),
    )
}
