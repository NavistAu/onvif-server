//! WS-Discovery UDP multicast listener.
//! Only compiled when the `discovery` feature is enabled.

#[cfg(feature = "discovery")]
pub async fn run_discovery(xaddr: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        // Only respond to Probe messages — check for wsdd:Probe element name in body bytes
        if msg.windows(5).any(|w| w == b"Probe") {
            let msg_id =
                extract_message_id(msg).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let reply = build_probe_match(&msg_id, &xaddr);
            let _ = udp.send_to(reply.as_bytes(), src).await;
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

#[cfg(feature = "discovery")]
fn build_probe_match(relates_to: &str, xaddr: &str) -> String {
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
        <a:EndpointReference><a:Address>urn:uuid:{msg_id}</a:Address></a:EndpointReference>
        <d:Types>dn:NetworkVideoTransmitter tds:Device</d:Types>
        <d:Scopes>onvif://www.onvif.org/type/NetworkVideoTransmitter</d:Scopes>
        <d:XAddrs>{xaddr}</d:XAddrs>
        <d:MetadataVersion>1</d:MetadataVersion>
      </d:ProbeMatch>
    </d:ProbeMatches>
  </s:Body>
</s:Envelope>"#,
        msg_id = msg_id,
        relates_to = relates_to,
        xaddr = xaddr
    )
}
