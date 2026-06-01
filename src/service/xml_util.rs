//! Shared XML parsing helpers for ONVIF service handlers.
//!
//! All helpers are namespace-aware (reject same-local-name elements in the
//! wrong namespace) and decode XML entities so trait implementations receive
//! the actual character values rather than `&amp;`, `&lt;`, etc.

use bytes::Bytes;
use quick_xml::events::Event;
use quick_xml::NsReader;
use soap_server::SoapFault;

/// Resolve a standard predefined XML entity name to its character.
/// Returns `None` for unknown names (caller should leave them as-is or error).
fn resolve_predefined_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "apos" => Some('\''),
        "quot" => Some('"'),
        _ => None,
    }
}

/// Extract the decoded text content of the first element that has the given
/// `element_name` as its local name AND whose namespace matches one of the
/// supplied `accepted_namespaces`.  Elements with no namespace binding (unbound)
/// are also accepted to support bare-local-name test payloads.
///
/// XML entities (`&amp;`, `&lt;`, `&#65;`, etc.) are decoded to their character
/// values before being returned, so callers receive the actual string content.
pub fn extract_text_ns(
    body: &Bytes,
    element_name: &str,
    accepted_namespaces: &[&[u8]],
) -> Result<String, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    let mut inside_target = false;
    let mut accumulated = String::new();

    loop {
        match reader
            .read_resolved_event()
            .map_err(|e| SoapFault::sender(format!("{e}")))?
        {
            (ns, Event::Start(e)) => {
                let local_name = e.local_name();
                let local = std::str::from_utf8(local_name.as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?;
                if local == element_name {
                    let ns_matches = matches!(
                        ns,
                        quick_xml::name::ResolveResult::Bound(ref n)
                            if accepted_namespaces.iter().any(|&a| n.as_ref() == a)
                    ) || matches!(ns, quick_xml::name::ResolveResult::Unbound);
                    if ns_matches {
                        inside_target = true;
                        accumulated.clear();
                    }
                } else if inside_target {
                    // Child element start — clear accumulated since we want only direct text
                    // (reset inside_target on any child start to avoid grabbing nested text)
                    inside_target = false;
                }
            }
            (_, Event::End(_)) => {
                if inside_target {
                    // End of the target element — return what we accumulated.
                    return Ok(accumulated);
                }
            }
            (_, Event::Text(t)) if inside_target => {
                let chunk = t
                    .decode()
                    .map_err(|e| SoapFault::sender(format!("text decode error: {e}")))?;
                accumulated.push_str(&chunk);
            }
            (_, Event::GeneralRef(r)) if inside_target => {
                // quick-xml 0.39 emits `&name;` as a GeneralRef event rather than
                // including it in the adjacent Text event.  We resolve the five
                // predefined XML entities and numeric character references here.
                let ref_name = r
                    .decode()
                    .map_err(|e| SoapFault::sender(format!("ref decode error: {e}")))?;
                // Numeric character reference?  BytesRef::resolve_char_ref handles these.
                if let Some(ch) = r
                    .resolve_char_ref()
                    .map_err(|e| SoapFault::sender(format!("char ref error: {e}")))?
                {
                    accumulated.push(ch);
                } else if let Some(ch) = resolve_predefined_entity(&ref_name) {
                    accumulated.push(ch);
                } else {
                    // Unknown entity — pass through unexpanded (best-effort).
                    accumulated.push('&');
                    accumulated.push_str(&ref_name);
                    accumulated.push(';');
                }
            }
            (_, Event::Eof) => {
                return Err(SoapFault::sender(format!(
                    "Element {element_name} not found in body"
                )))
            }
            _ => {}
        }
    }
}
