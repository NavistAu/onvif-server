//! Namespace-preserving SOAP Body child extractor.
//!
//! Ported from the algorithm in `soap-server::envelope::extract_body_first_child` +
//! `collect_ns_from_attrs`. Does NOT depend on soap_server internals.
//!
//! The key invariant: namespace declarations from ancestor elements (Envelope, Body)
//! are re-emitted onto the extracted body child so that any prefix used inside the
//! child element (e.g. `tt:`, `tds:`, `tptz:`) remains resolvable when the child
//! bytes are validated in isolation by the oracle.

use quick_xml::events::Event;
use quick_xml::Reader;

/// Extract the first child element of `soap:Body` from a full SOAP envelope,
/// re-emitting all in-scope ancestor namespace declarations onto the child root.
///
/// Returns `None` if the Body is absent or empty.
pub fn extract_body_child(envelope: &[u8]) -> Option<Vec<u8>> {
    // Phase 1: single-pass scan collecting namespace bindings as we descend.
    // We use a simple Reader (not NsReader) so we can capture the raw attribute bytes.

    let mut reader = Reader::from_reader(envelope);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    // Accumulated namespace bindings from Envelope + Body elements.
    // Format: Vec<(prefix, uri)> where prefix="" means the default namespace.
    // New bindings are prepended so element-own declarations shadow ancestor ones.
    let mut ns_bindings: Vec<(String, String)> = Vec::new();

    let mut in_body = false;

    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Eof => return None,

            Event::Start(ref e) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = local_name(&name_bytes);

                if !in_body {
                    if local == b"Envelope" {
                        collect_ns(e, &mut ns_bindings);
                    } else if local == b"Body" {
                        collect_ns(e, &mut ns_bindings);
                        in_body = true;
                    }
                } else {
                    // First child of Body — build the self-contained child bytes.
                    return Some(build_child_bytes(
                        &name_bytes,
                        e,
                        &ns_bindings,
                        &mut reader,
                        false,
                    ));
                }
            }

            Event::Empty(ref e) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = local_name(&name_bytes);

                if in_body {
                    // Self-closing first child of Body.
                    return Some(build_empty_child_bytes(&name_bytes, e, &ns_bindings));
                } else if local == b"Body" {
                    // <Body/> with no children.
                    return None;
                }
            }

            _ => {}
        }
        buf.clear();
    }
}

/// Extract the local name (after the last `:`) from a qualified name byte slice.
fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

/// Collect xmlns declarations from a start element's attributes into `bindings`.
/// New bindings are prepended so they shadow inherited ones.
fn collect_ns(e: &quick_xml::events::BytesStart<'_>, bindings: &mut Vec<(String, String)>) {
    let mut new: Vec<(String, String)> = Vec::new();
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        if key.starts_with("xmlns:") {
            let prefix = key.trim_start_matches("xmlns:").to_string();
            let uri = std::str::from_utf8(attr.value.as_ref())
                .unwrap_or("")
                .to_string();
            new.push((prefix, uri));
        } else if key == "xmlns" {
            let uri = std::str::from_utf8(attr.value.as_ref())
                .unwrap_or("")
                .to_string();
            new.push((String::new(), uri));
        }
    }
    // Prepend so element-own declarations take precedence over inherited ones.
    new.append(bindings);
    *bindings = new;
}

/// Build the byte representation of the body child start element + its content,
/// re-emitting ancestor namespace declarations not already declared by the child.
fn build_child_bytes(
    name_bytes: &[u8],
    e: &quick_xml::events::BytesStart<'_>,
    ns_bindings: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    _empty: bool,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(b'<');
    buf.extend_from_slice(name_bytes);

    // Collect child's own xmlns declarations to avoid re-emitting them.
    let mut own_prefixes: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut attr_bytes: Vec<u8> = Vec::new();

    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        if key.starts_with("xmlns:") {
            own_prefixes.insert(key.trim_start_matches("xmlns:").to_string());
        } else if key == "xmlns" {
            own_prefixes.insert(String::new());
        }
        attr_bytes.push(b' ');
        attr_bytes.extend_from_slice(attr.key.as_ref());
        attr_bytes.extend_from_slice(b"=\"");
        attr_bytes.extend_from_slice(attr.value.as_ref());
        attr_bytes.push(b'"');
    }

    // Re-emit ancestor ns declarations not overridden by the child.
    // Track which prefixes we have already emitted to avoid duplicates from ancestor chain.
    let mut emitted: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (prefix, uri) in ns_bindings {
        if own_prefixes.contains(prefix.as_str()) {
            continue;
        }
        if emitted.contains(prefix.as_str()) {
            continue;
        }
        emitted.insert(prefix.clone());
        if prefix.is_empty() {
            buf.extend_from_slice(b" xmlns=\"");
        } else {
            buf.extend_from_slice(b" xmlns:");
            buf.extend_from_slice(prefix.as_bytes());
            buf.extend_from_slice(b"=\"");
        }
        buf.extend_from_slice(uri.as_bytes());
        buf.push(b'"');
    }

    buf.extend_from_slice(&attr_bytes);
    buf.push(b'>');

    // Collect remaining content until the matching end tag.
    let mut depth = 1i32;
    let mut rbuf = Vec::new();
    loop {
        match reader.read_event_into(&mut rbuf) {
            Ok(Event::Start(ref e2)) => {
                depth += 1;
                buf.push(b'<');
                buf.extend_from_slice(e2.name().as_ref());
                for attr in e2.attributes().flatten() {
                    buf.push(b' ');
                    buf.extend_from_slice(attr.key.as_ref());
                    buf.extend_from_slice(b"=\"");
                    buf.extend_from_slice(attr.value.as_ref());
                    buf.push(b'"');
                }
                buf.push(b'>');
            }
            Ok(Event::Empty(ref e2)) => {
                buf.push(b'<');
                buf.extend_from_slice(e2.name().as_ref());
                for attr in e2.attributes().flatten() {
                    buf.push(b' ');
                    buf.extend_from_slice(attr.key.as_ref());
                    buf.extend_from_slice(b"=\"");
                    buf.extend_from_slice(attr.value.as_ref());
                    buf.push(b'"');
                }
                buf.extend_from_slice(b"/>");
            }
            Ok(Event::End(ref e2)) => {
                depth -= 1;
                buf.extend_from_slice(b"</");
                buf.extend_from_slice(e2.name().as_ref());
                buf.push(b'>');
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Text(ref t)) => {
                buf.extend_from_slice(t.as_ref());
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        rbuf.clear();
    }

    buf
}

/// Build the byte representation of an empty (self-closing) body child element,
/// re-emitting ancestor namespace declarations.
fn build_empty_child_bytes(
    name_bytes: &[u8],
    e: &quick_xml::events::BytesStart<'_>,
    ns_bindings: &[(String, String)],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(b'<');
    buf.extend_from_slice(name_bytes);

    let mut own_prefixes: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut attr_bytes: Vec<u8> = Vec::new();

    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        if key.starts_with("xmlns:") {
            own_prefixes.insert(key.trim_start_matches("xmlns:").to_string());
        } else if key == "xmlns" {
            own_prefixes.insert(String::new());
        }
        attr_bytes.push(b' ');
        attr_bytes.extend_from_slice(attr.key.as_ref());
        attr_bytes.extend_from_slice(b"=\"");
        attr_bytes.extend_from_slice(attr.value.as_ref());
        attr_bytes.push(b'"');
    }

    let mut emitted: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (prefix, uri) in ns_bindings {
        if own_prefixes.contains(prefix.as_str()) {
            continue;
        }
        if emitted.contains(prefix.as_str()) {
            continue;
        }
        emitted.insert(prefix.clone());
        if prefix.is_empty() {
            buf.extend_from_slice(b" xmlns=\"");
        } else {
            buf.extend_from_slice(b" xmlns:");
            buf.extend_from_slice(prefix.as_bytes());
            buf.extend_from_slice(b"=\"");
        }
        buf.extend_from_slice(uri.as_bytes());
        buf.push(b'"');
    }

    buf.extend_from_slice(&attr_bytes);
    buf.extend_from_slice(b"/>");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_body_child_with_ancestor_ns_declarations() {
        // xmlns:tt declared on Envelope, <tt:Foo> as body child.
        // The extracted child MUST carry xmlns:tt.
        let envelope = br#"<s:Envelope
            xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tt="http://www.onvif.org/ver10/schema">
          <s:Body>
            <tt:Foo><tt:Bar>hello</tt:Bar></tt:Foo>
          </s:Body>
        </s:Envelope>"#;

        let child = extract_body_child(envelope).expect("should extract child");
        let child_str = String::from_utf8(child).unwrap();

        assert!(
            child_str.contains("xmlns:tt=\"http://www.onvif.org/ver10/schema\""),
            "extracted child must carry xmlns:tt; got: {child_str}"
        );
        assert!(
            child_str.contains("<tt:Bar>hello</tt:Bar>"),
            "extracted child must contain inner content; got: {child_str}"
        );
        assert!(
            child_str.starts_with("<tt:Foo"),
            "extracted root must be tt:Foo; got: {child_str}"
        );
    }

    #[test]
    fn extracts_body_child_with_multiple_ancestor_ns() {
        // xmlns:tds and xmlns:tt on Envelope; body child uses tds:
        let envelope = br#"<s:Envelope
            xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
            xmlns:tt="http://www.onvif.org/ver10/schema">
          <s:Body>
            <tds:GetDeviceInformationResponse>
              <tt:Manufacturer>ACME</tt:Manufacturer>
            </tds:GetDeviceInformationResponse>
          </s:Body>
        </s:Envelope>"#;

        let child = extract_body_child(envelope).expect("should extract child");
        let child_str = String::from_utf8(child).unwrap();

        assert!(
            child_str.contains("xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\""),
            "must carry xmlns:tds; got: {child_str}"
        );
        assert!(
            child_str.contains("xmlns:tt=\"http://www.onvif.org/ver10/schema\""),
            "must carry xmlns:tt; got: {child_str}"
        );
    }

    #[test]
    fn empty_body_returns_none() {
        let envelope = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
          <s:Body/>
        </s:Envelope>"#;
        assert!(extract_body_child(envelope).is_none());
    }

    #[test]
    fn child_own_ns_not_duplicated() {
        // If the child element already declares xmlns:tt, we must NOT re-emit it from ancestor.
        let envelope = br#"<s:Envelope
            xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tt="http://www.onvif.org/ver10/schema">
          <s:Body>
            <tt:Foo xmlns:tt="http://www.onvif.org/ver10/schema"/>
          </s:Body>
        </s:Envelope>"#;

        let child = extract_body_child(envelope).expect("should extract child");
        let child_str = String::from_utf8(child).unwrap();
        // Count occurrences — must be exactly one.
        let count = child_str.matches("xmlns:tt=").count();
        assert_eq!(
            count, 1,
            "xmlns:tt must appear exactly once; got: {child_str}"
        );
    }
}
