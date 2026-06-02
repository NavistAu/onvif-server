//! Layer-1 normalization (spec §5.3): parse → path-scoped DOM mask →
//! deterministic serialize. This is REGRESSION canonicalization (our output vs our
//! own frozen baseline), NOT the authoritative C14N — that is delegated to the Java
//! XML oracle in Layer 1b and never done here.
//!
//! Layer-2 normalization primitives are also here:
//! - `canonicalize_prefixes`: rewrite namespace prefixes to deterministic `n0`, `n1`, …
//! - `AttrMaskRule`: path-scoped attribute mask (drops an attribute at a given path)
//! - `mask_only`: apply text + attr masks then prefix-canon; returns bytes for C14N oracle.

use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, Reader, Writer};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

pub const MASK_SENTINEL: &str = "__MASKED__";

/// A path-scoped mask: the slash-joined local-name path whose text is masked.
#[derive(Debug, Clone)]
pub struct MaskRule {
    segments: Vec<String>,
}

impl MaskRule {
    pub fn new(path: &str) -> Self {
        MaskRule {
            segments: path.split('/').map(|s| s.to_string()).collect(),
        }
    }
    fn matches(&self, stack: &[String]) -> bool {
        stack.len() == self.segments.len() && stack.iter().zip(&self.segments).all(|(a, b)| a == b)
    }
}

fn local_name(e: &BytesStart) -> String {
    let full = e.name();
    let bytes = full.as_ref();
    let local = match bytes.iter().rposition(|&b| b == b':') {
        Some(i) => &bytes[i + 1..],
        None => bytes,
    };
    String::from_utf8_lossy(local).into_owned()
}

/// Re-serialize a start tag with attributes sorted by full QName (stable output).
/// NOTE: sorting by raw QName bytes is stable only while namespace prefix assignments
/// are deterministic (true for the controlled SUT; Task 7's ns_prefix_shadowing
/// scenarios remain within that boundary).
fn write_sorted_start<W: std::io::Write>(
    w: &mut Writer<W>,
    e: &BytesStart,
    empty: bool,
) -> Result<(), String> {
    let mut elem = BytesStart::new(String::from_utf8_lossy(e.name().as_ref()).into_owned());
    // Collect raw key/value bytes so we can sort by key without re-escaping values.
    let mut attrs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|err| err.to_string())?;
        attrs.push((a.key.as_ref().to_owned(), a.value.into_owned()));
    }
    // Sort by key bytes for deterministic output.
    attrs.sort_by(|x, y| x.0.cmp(&y.0));
    for (k, v) in &attrs {
        // From<(&[u8], &[u8])> stores the value verbatim — no re-escape pass.
        elem.push_attribute((k.as_slice(), v.as_slice()));
    }
    let ev = if empty {
        Event::Empty(elem)
    } else {
        Event::Start(elem)
    };
    w.write_event(ev).map_err(|err| err.to_string())
}

/// Parse `xml`, mask path-scoped element text, and emit deterministic output.
pub fn normalize(xml: &[u8], masks: &[MaskRule]) -> Result<String, String> {
    let mut reader = Reader::from_reader(xml);
    // check_end_names defaults to true in quick-xml 0.39 — mismatched end tags
    // surface as an error from read_event_into, satisfying the malformed_xml_errors test.
    // trim_text(false): quick-xml fragments text around entity refs; trimming edges of each
    // fragment silently drops significant whitespace adjacent to entities (e.g. spaces around
    // &apos; or &amp;). soap-server emits compact XML so indentation whitespace is not a concern.
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(e) => {
                stack.push(local_name(&e));
                write_sorted_start(&mut writer, &e, false)?;
            }
            Event::Empty(e) => {
                stack.push(local_name(&e));
                write_sorted_start(&mut writer, &e, true)?;
                stack.pop();
            }
            Event::End(e) => {
                writer
                    .write_event(Event::End(e.to_owned()))
                    .map_err(|err| err.to_string())?;
                stack.pop();
            }
            Event::Text(t) => {
                let masked = masks.iter().any(|m| m.matches(&stack));
                if masked {
                    writer
                        .write_event(Event::Text(quick_xml::events::BytesText::new(
                            MASK_SENTINEL,
                        )))
                        .map_err(|err| err.to_string())?;
                } else {
                    writer
                        .write_event(Event::Text(t.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::CData(cd) => {
                let masked = masks.iter().any(|m| m.matches(&stack));
                if masked {
                    writer
                        .write_event(Event::Text(BytesText::new(MASK_SENTINEL)))
                        .map_err(|err| err.to_string())?;
                } else {
                    writer
                        .write_event(Event::CData(cd.into_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::Eof => break,
            other => {
                writer
                    .write_event(other.to_owned())
                    .map_err(|err| err.to_string())?;
            }
        }
        buf.clear();
    }
    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

// ─── Layer-2 primitives ──────────────────────────────────────────────────────

/// The `xml:` prefix namespace URI — must never be reassigned (XML spec).
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// Rewrite all element/attribute QNames to use a deterministic prefix per namespace
/// URI (assigned by sorted URI), so two documents that are namespace-equivalent but use
/// different prefixes (or default-ns vs a prefix) serialize identically. Prefix choice
/// is not semantically meaningful in XML; this is structural normalization, not grading.
pub fn canonicalize_prefixes(xml: &[u8]) -> Result<Vec<u8>, String> {
    // ── First pass: collect all namespace URIs actually used by elements + attrs ─
    let used_uris = collect_used_uris(xml)?;

    // ── Build prefix map: BTreeSet iterates in sorted order → n0, n1, … ─────
    let uri_to_prefix: BTreeMap<String, String> = used_uris
        .iter()
        .enumerate()
        .map(|(i, uri)| (uri.clone(), format!("n{i}")))
        .collect();

    // ── Second pass: re-serialize with canonical prefixes ────────────────────
    // We need a stack of (local_name, canonical_prefix) to emit matching end tags.
    let mut reader = NsReader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut out = Vec::with_capacity(xml.len() + 128);
    let mut buf = Vec::new();
    let mut first_element = true;
    // Stack entries: (local_name_bytes, canonical_prefix)  — both owned Strings.
    let mut elem_stack: Vec<(String, String)> = Vec::new();

    loop {
        match reader
            .read_resolved_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            (resolve, Event::Start(ref e)) => {
                let cpfx = resolved_canonical_prefix(&resolve, &uri_to_prefix)?;
                let local = local_name_str(e.name().as_ref());
                emit_canon_start(
                    &mut out,
                    e,
                    &cpfx,
                    &local,
                    &uri_to_prefix,
                    &reader,
                    first_element,
                    false,
                )?;
                elem_stack.push((local, cpfx));
                first_element = false;
            }
            (resolve, Event::Empty(ref e)) => {
                let cpfx = resolved_canonical_prefix(&resolve, &uri_to_prefix)?;
                let local = local_name_str(e.name().as_ref());
                emit_canon_start(
                    &mut out,
                    e,
                    &cpfx,
                    &local,
                    &uri_to_prefix,
                    &reader,
                    first_element,
                    true,
                )?;
                first_element = false;
                // Empty elements have no end tag — don't push/pop stack.
            }
            (_, Event::End(_)) => {
                // Use our stack (not the raw event bytes) to emit the correct prefix.
                let (local, cpfx) = elem_stack
                    .pop()
                    .ok_or_else(|| "end tag without matching start".to_string())?;
                out.extend_from_slice(b"</");
                if !cpfx.is_empty() {
                    out.extend_from_slice(cpfx.as_bytes());
                    out.push(b':');
                }
                out.extend_from_slice(local.as_bytes());
                out.push(b'>');
            }
            (_, Event::Text(ref t)) => {
                out.extend_from_slice(t.as_ref());
            }
            (_, Event::CData(ref cd)) => {
                out.extend_from_slice(b"<![CDATA[");
                out.extend_from_slice(cd.as_ref());
                out.extend_from_slice(b"]]>");
            }
            // Re-emit entity references verbatim so the oracle C14N can normalise them.
            // Without this, &apos; / &lt; / etc. in element text are silently dropped.
            (_, Event::GeneralRef(ref r)) => {
                out.push(b'&');
                out.extend_from_slice(r.as_ref());
                out.push(b';');
            }
            (_, Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

/// First pass: walk the document with NsReader and collect every namespace URI
/// actually bound on an element or prefixed attribute (excluding `xml:`).
fn collect_used_uris(xml: &[u8]) -> Result<BTreeSet<String>, String> {
    let mut uris: BTreeSet<String> = BTreeSet::new();
    let mut reader = NsReader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader
            .read_resolved_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            (ResolveResult::Bound(ns), Event::Start(ref e))
            | (ResolveResult::Bound(ns), Event::Empty(ref e)) => {
                let uri = String::from_utf8_lossy(ns.as_ref()).into_owned();
                if uri != XML_NS {
                    uris.insert(uri);
                }
                collect_attr_uris(&mut uris, e, &reader)?;
            }
            (ResolveResult::Unbound, Event::Start(ref e))
            | (ResolveResult::Unbound, Event::Empty(ref e)) => {
                collect_attr_uris(&mut uris, e, &reader)?;
            }
            (_, Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(uris)
}

fn collect_attr_uris(
    uris: &mut BTreeSet<String>,
    e: &BytesStart,
    reader: &NsReader<&[u8]>,
) -> Result<(), String> {
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(|e| e.to_string())?;
        let key = attr.key.as_ref();
        if key == b"xmlns" || key.starts_with(b"xmlns:") {
            continue;
        }
        // xml: prefix is reserved — do not add to used_uris.
        if key.starts_with(b"xml:") {
            continue;
        }
        if let (ResolveResult::Bound(ans), _) = reader.resolver().resolve_attribute(attr.key) {
            let auri = String::from_utf8_lossy(ans.as_ref()).into_owned();
            if auri != XML_NS {
                uris.insert(auri);
            }
        }
    }
    Ok(())
}

/// Map a ResolveResult to a canonical prefix string (empty string = no namespace).
fn resolved_canonical_prefix(
    resolve: &ResolveResult,
    uri_to_prefix: &BTreeMap<String, String>,
) -> Result<String, String> {
    match resolve {
        ResolveResult::Bound(ns) => {
            let uri = String::from_utf8_lossy(ns.as_ref()).into_owned();
            uri_to_prefix
                .get(&uri)
                .cloned()
                .ok_or_else(|| format!("unmapped namespace URI: {uri}"))
        }
        ResolveResult::Unbound => Ok(String::new()),
        ResolveResult::Unknown(pfx) => {
            // Should not happen for well-formed XML, but surface the error.
            Err(format!(
                "unknown namespace prefix: {}",
                String::from_utf8_lossy(pfx)
            ))
        }
    }
}

fn local_name_str(raw: &[u8]) -> String {
    let local = match raw.iter().rposition(|&b| b == b':') {
        Some(i) => &raw[i + 1..],
        None => raw,
    };
    String::from_utf8_lossy(local).into_owned()
}

/// Emit a start (or empty-element) tag with canonical prefix, root xmlns declarations,
/// and rewritten attribute prefixes.
#[allow(clippy::too_many_arguments)]
fn emit_canon_start(
    out: &mut Vec<u8>,
    e: &BytesStart,
    cpfx: &str,
    local: &str,
    uri_to_prefix: &BTreeMap<String, String>,
    reader: &NsReader<&[u8]>,
    is_root: bool,
    is_empty: bool,
) -> Result<(), String> {
    out.push(b'<');
    if !cpfx.is_empty() {
        out.extend_from_slice(cpfx.as_bytes());
        out.push(b':');
    }
    out.extend_from_slice(local.as_bytes());

    // On root element, emit ALL namespace declarations sorted by prefix (n0, n1, …).
    if is_root {
        // uri_to_prefix is a BTreeMap<uri, prefix>; iterate sorted by prefix name.
        // n0 < n1 < … because BTreeMap sorts by key (URI), but we want sorted by prefix.
        // Collect and sort by prefix string.
        let mut decls: Vec<(&str, &str)> = uri_to_prefix
            .iter()
            .map(|(uri, pfx)| (uri.as_str(), pfx.as_str()))
            .collect();
        decls.sort_by_key(|(_, pfx)| *pfx);
        for (uri, pfx) in decls {
            out.push(b' ');
            out.extend_from_slice(b"xmlns:");
            out.extend_from_slice(pfx.as_bytes());
            out.extend_from_slice(b"=\"");
            out.extend_from_slice(uri.as_bytes());
            out.push(b'"');
        }
    }

    // Emit attributes: skip xmlns declarations, preserve xml: attrs, rewrite others.
    let mut attrs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(|e| e.to_string())?;
        let key = attr.key.as_ref();

        // Skip xmlns declarations (hoisted to root or dropped).
        if key == b"xmlns" || key.starts_with(b"xmlns:") {
            continue;
        }

        // Preserve xml:* attributes verbatim (xml: is a reserved prefix).
        if key.starts_with(b"xml:") {
            attrs.push((key.to_owned(), attr.value.into_owned()));
            continue;
        }

        // Prefixed attribute: rewrite with canonical prefix.
        if key.contains(&b':') {
            let colon = key.iter().position(|&b| b == b':').unwrap();
            let local_part = &key[colon + 1..];
            if let (ResolveResult::Bound(ans), _) = reader.resolver().resolve_attribute(attr.key) {
                let auri = String::from_utf8_lossy(ans.as_ref()).into_owned();
                if let Some(new_pfx) = uri_to_prefix.get(&auri) {
                    let mut new_key = new_pfx.as_bytes().to_vec();
                    new_key.push(b':');
                    new_key.extend_from_slice(local_part);
                    attrs.push((new_key, attr.value.into_owned()));
                    continue;
                }
            }
        }

        // Unprefixed attribute (no namespace) — keep as-is.
        attrs.push((key.to_owned(), attr.value.into_owned()));
    }

    // Sort for deterministic output.
    attrs.sort_by(|a, b| a.0.cmp(&b.0));
    for (k, v) in &attrs {
        out.push(b' ');
        out.extend_from_slice(k);
        out.extend_from_slice(b"=\"");
        write_attr_value(out, v);
        out.push(b'"');
    }

    if is_empty {
        out.extend_from_slice(b"/>");
    } else {
        out.push(b'>');
    }
    Ok(())
}

fn write_attr_value(out: &mut Vec<u8>, v: &[u8]) {
    for &byte in v {
        match byte {
            b'"' => out.extend_from_slice(b"&quot;"),
            b'<' => out.extend_from_slice(b"&lt;"),
            b'&' => out.extend_from_slice(b"&amp;"),
            other => out.push(other),
        }
    }
}

// ─── AttrMaskRule ─────────────────────────────────────────────────────────────

/// Path-scoped attribute mask: at the element whose local-name path matches `path`,
/// the attribute named `attr` (by its full written name, e.g. "xml:lang") is dropped
/// from the serialized output (so cross-impl attribute-value differences on
/// non-asserted nodes don't cause false diffs).
#[derive(Debug, Clone)]
pub struct AttrMaskRule {
    segments: Vec<String>,
    attr: String,
}

impl AttrMaskRule {
    pub fn new(path: &str, attr: &str) -> Self {
        AttrMaskRule {
            segments: path.split('/').map(|s| s.to_string()).collect(),
            attr: attr.to_string(),
        }
    }

    fn matches_path(&self, stack: &[String]) -> bool {
        stack.len() == self.segments.len() && stack.iter().zip(&self.segments).all(|(a, b)| a == b)
    }

    fn should_drop_attr(&self, stack: &[String], attr_name: &str) -> bool {
        self.matches_path(stack) && self.attr == attr_name
    }
}

// ─── mask_only ────────────────────────────────────────────────────────────────

/// Layer-2 normalization: apply path-scoped text masks + path-scoped attribute masks,
/// then canonicalize namespace prefixes. Returns bytes for the Java XML oracle to
/// exclusive-C14N (this fn does NOT canonicalize — C14N is the oracle's authority).
pub fn mask_only(
    xml: &[u8],
    text_masks: &[MaskRule],
    attr_masks: &[AttrMaskRule],
) -> Result<Vec<u8>, String> {
    // Step 1: apply masks (text + attribute) and re-serialize.
    let masked = apply_masks(xml, text_masks, attr_masks)?;
    // Step 2: canonicalize namespace prefixes on the result.
    canonicalize_prefixes(&masked)
}

/// Like `mask_only` but also drops entire subtrees whose local-name path matches
/// any entry in `drop_paths`. The element and all its descendants are omitted from
/// output. This is used to strip `Envelope/Header` from WS-Security responses so
/// the response `wsse:Security`/Timestamp added by CXF does not affect body comparison.
pub fn mask_only_with_drops(
    xml: &[u8],
    text_masks: &[MaskRule],
    attr_masks: &[AttrMaskRule],
    drop_paths: &[MaskRule],
) -> Result<Vec<u8>, String> {
    let dropped = drop_subtrees(xml, drop_paths)?;
    let masked = apply_masks(&dropped, text_masks, attr_masks)?;
    canonicalize_prefixes(&masked)
}

/// Parse `xml` and omit any element (and its entire subtree) whose local-name path
/// matches a rule in `drop_paths`.
fn drop_subtrees(xml: &[u8], drop_paths: &[MaskRule]) -> Result<Vec<u8>, String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut stack: Vec<String> = Vec::new();
    // Depth at which we entered a dropped subtree (None = not dropping).
    let mut drop_depth: Option<usize> = None;
    let mut buf = Vec::new();

    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(ref e) => {
                let lname = local_name(e);
                stack.push(lname);
                if drop_depth.is_some() {
                    // Already inside a dropped subtree — don't emit.
                } else if drop_paths.iter().any(|r| r.matches(&stack)) {
                    // Entering a dropped subtree — record depth, don't emit.
                    drop_depth = Some(stack.len());
                } else {
                    writer
                        .write_event(Event::Start(e.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::Empty(ref e) => {
                let lname = local_name(e);
                stack.push(lname.clone());
                if drop_depth.is_none() {
                    if drop_paths.iter().any(|r| r.matches(&stack)) {
                        // Empty element at a drop path — silently skip.
                    } else {
                        writer
                            .write_event(Event::Empty(e.to_owned()))
                            .map_err(|err| err.to_string())?;
                    }
                }
                stack.pop();
            }
            Event::End(ref e) => {
                if let Some(dd) = drop_depth {
                    if stack.len() == dd {
                        // Closing tag of the dropped root element — exit drop mode.
                        drop_depth = None;
                    }
                    // Don't emit the end tag (it's inside the dropped subtree).
                } else {
                    writer
                        .write_event(Event::End(e.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
                stack.pop();
            }
            Event::Text(ref t) => {
                if drop_depth.is_none() {
                    writer
                        .write_event(Event::Text(t.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::CData(ref cd) => {
                if drop_depth.is_none() {
                    writer
                        .write_event(Event::CData(cd.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::GeneralRef(ref r) => {
                if drop_depth.is_none() {
                    writer
                        .write_event(Event::GeneralRef(r.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::Eof => break,
            other => {
                if drop_depth.is_none() {
                    writer
                        .write_event(other.to_owned())
                        .map_err(|err| err.to_string())?;
                }
            }
        }
        buf.clear();
    }
    Ok(writer.into_inner().into_inner())
}

/// Parse `xml`, apply text masks and attribute masks, re-serialize to bytes.
fn apply_masks(
    xml: &[u8],
    text_masks: &[MaskRule],
    attr_masks: &[AttrMaskRule],
) -> Result<Vec<u8>, String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut stack: Vec<String> = Vec::new();
    // Whether we have already written the __MASKED__ sentinel for the current
    // masked element's content. Reset on every End tag at a masked path.
    let mut sentinel_written = false;
    let mut buf = Vec::new();

    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(ref e) => {
                let lname = local_name(e);
                stack.push(lname);
                write_masked_start(&mut writer, e, &stack, attr_masks, false)?;
                // Entering a new element resets the sentinel flag.
                sentinel_written = false;
            }
            Event::Empty(ref e) => {
                let lname = local_name(e);
                stack.push(lname.clone());
                write_masked_start(&mut writer, e, &stack, attr_masks, true)?;
                stack.pop();
                sentinel_written = false;
            }
            Event::End(ref e) => {
                writer
                    .write_event(Event::End(e.to_owned()))
                    .map_err(|err| err.to_string())?;
                stack.pop();
                sentinel_written = false;
            }
            Event::Text(ref t) => {
                let at_masked = text_masks.iter().any(|m| m.matches(&stack));
                if at_masked {
                    if !sentinel_written {
                        writer
                            .write_event(Event::Text(BytesText::new(MASK_SENTINEL)))
                            .map_err(|err| err.to_string())?;
                        sentinel_written = true;
                    }
                    // Subsequent Text/GeneralRef/CData events in this element are dropped.
                } else {
                    writer
                        .write_event(Event::Text(t.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::CData(ref cd) => {
                let at_masked = text_masks.iter().any(|m| m.matches(&stack));
                if at_masked {
                    if !sentinel_written {
                        writer
                            .write_event(Event::Text(BytesText::new(MASK_SENTINEL)))
                            .map_err(|err| err.to_string())?;
                        sentinel_written = true;
                    }
                } else {
                    writer
                        .write_event(Event::CData(cd.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            // Entity refs within a masked element must be suppressed so they don't
            // leak between __MASKED__ sentinels (e.g. &apos; in fault reason text).
            Event::GeneralRef(ref r) => {
                let at_masked = text_masks.iter().any(|m| m.matches(&stack));
                if at_masked {
                    // Drop entity refs at masked paths (the whole content is replaced by sentinel).
                } else {
                    writer
                        .write_event(Event::GeneralRef(r.to_owned()))
                        .map_err(|err| err.to_string())?;
                }
            }
            Event::Eof => break,
            other => {
                writer
                    .write_event(other.to_owned())
                    .map_err(|err| err.to_string())?;
            }
        }
        buf.clear();
    }
    Ok(writer.into_inner().into_inner())
}

/// Write a start (or empty) tag, dropping any attributes matched by attr_masks at this path.
fn write_masked_start<W: std::io::Write>(
    w: &mut Writer<W>,
    e: &BytesStart,
    stack: &[String],
    attr_masks: &[AttrMaskRule],
    is_empty: bool,
) -> Result<(), String> {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut elem = BytesStart::new(name);
    let mut attrs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|err| err.to_string())?;
        let key_str = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        // Check attr masks.
        let dropped = attr_masks
            .iter()
            .any(|m| m.should_drop_attr(stack, &key_str));
        if !dropped {
            attrs.push((a.key.as_ref().to_owned(), a.value.into_owned()));
        }
    }
    // Sort for stability (matches existing normalize() behavior).
    attrs.sort_by(|x, y| x.0.cmp(&y.0));
    for (k, v) in &attrs {
        elem.push_attribute((k.as_slice(), v.as_slice()));
    }
    let ev = if is_empty {
        Event::Empty(elem)
    } else {
        Event::Start(elem)
    };
    w.write_event(ev).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_only_the_path_scoped_element_text() {
        let xml = br#"<Envelope><Header><Nonce>AAAA</Nonce></Header><Body><Nonce>keepme</Nonce></Body></Envelope>"#;
        let rules = vec![MaskRule::new("Envelope/Header/Nonce")];
        let out = normalize(xml, &rules).unwrap();
        // Header Nonce masked, Body Nonce (different path) preserved.
        assert!(out.contains("<Nonce>__MASKED__</Nonce>"));
        assert!(out.contains("<Nonce>keepme</Nonce>"));
    }

    #[test]
    fn sorts_attributes_for_stable_output() {
        let a = normalize(br#"<E b="2" a="1"/>"#, &[]).unwrap();
        let b = normalize(br#"<E a="1" b="2"/>"#, &[]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn malformed_xml_errors() {
        assert!(normalize(b"<E></WRONG>", &[]).is_err());
    }

    #[test]
    fn attribute_values_are_not_double_escaped() {
        let out = normalize(br#"<E a="x &amp; y"/>"#, &[]).unwrap();
        assert!(out.contains("x &amp; y"), "got: {out}");
        assert!(!out.contains("&amp;amp;"), "double-escaped: {out}");
    }

    #[test]
    fn preserves_significant_whitespace_around_entities() {
        // quick-xml splits text at entity refs; normalization must NOT drop the
        // significant spaces adjacent to an entity (regression: trim_text was eating them).
        let xml = br#"<E>a &amp; b 'c' d</E>"#;
        let out = normalize(xml, &[]).unwrap();
        assert!(out.contains("a &amp; b"), "lost space around &amp;: {out}");
        // the ' characters re-serialize as &apos; (quick-xml writer) but spaces must remain:
        assert!(
            out.contains("b ") && out.contains(" d"),
            "lost surrounding spaces: {out}"
        );
    }

    #[test]
    fn masks_cdata_content_at_a_masked_path() {
        let xml = br#"<Envelope><Header><Nonce><![CDATA[AAAA]]></Nonce></Header></Envelope>"#;
        let rules = vec![MaskRule::new("Envelope/Header/Nonce")];
        let out = normalize(xml, &rules).unwrap();
        assert!(out.contains("__MASKED__"), "got: {out}");
        assert!(!out.contains("AAAA"), "unmasked CDATA leaked: {out}");
    }

    // ─── Part A: canonicalize_prefixes ────────────────────────────────────────

    #[test]
    fn prefix_and_default_ns_collapse_to_same_output() {
        let a =
            canonicalize_prefixes(br#"<c:Foo xmlns:c="urn:u"><c:Bar>x</c:Bar></c:Foo>"#).unwrap();
        let b = canonicalize_prefixes(br#"<Foo xmlns="urn:u"><Bar>x</Bar></Foo>"#).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn the_two_real_echo_responses_collapse_equal() {
        let ours = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><c:EchoResponse xmlns:c="http://crossref.example/controlled"><c:Text>hello</c:Text></c:EchoResponse></env:Body></env:Envelope>"#;
        let cxf = br#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><EchoResponse xmlns="http://crossref.example/controlled"><Text>hello</Text></EchoResponse></soap:Body></soap:Envelope>"#;
        assert_eq!(
            canonicalize_prefixes(ours).unwrap(),
            canonicalize_prefixes(cxf).unwrap()
        );
    }

    #[test]
    fn distinct_namespaces_get_distinct_prefixes() {
        let out =
            canonicalize_prefixes(br#"<a:X xmlns:a="urn:one" xmlns:b="urn:two"><b:Y/></a:X>"#)
                .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("urn:one") && s.contains("urn:two"));
    }

    #[test]
    fn xml_lang_keeps_xml_prefix() {
        let out = canonicalize_prefixes(br#"<a:X xmlns:a="urn:u" xml:lang="en"/>"#).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("xml:lang=\"en\""), "got {s}");
    }

    // ─── Part B: AttrMaskRule ─────────────────────────────────────────────────

    #[test]
    fn attr_mask_drops_named_attr_at_path() {
        let masks_txt: Vec<MaskRule> = vec![];
        let attr_masks = vec![AttrMaskRule::new(
            "Envelope/Body/Fault/Reason/Text",
            "xml:lang",
        )];
        let xml = br#"<Envelope><Body><Fault><Reason><Text xml:lang="en-US">boom</Text></Reason></Fault></Body></Envelope>"#;
        let out = String::from_utf8(mask_only(xml, &masks_txt, &attr_masks).unwrap()).unwrap();
        assert!(
            !out.contains("xml:lang"),
            "xml:lang should be dropped: {out}"
        );
    }

    // ─── Part C: mask_only end-to-end ─────────────────────────────────────────

    #[test]
    fn two_echo_responses_mask_only_equal() {
        let ours = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><c:EchoResponse xmlns:c="http://crossref.example/controlled"><c:Text>hello</c:Text></c:EchoResponse></env:Body></env:Envelope>"#;
        let cxf = br#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><EchoResponse xmlns="http://crossref.example/controlled"><Text>hello</Text></EchoResponse></soap:Body></soap:Envelope>"#;
        assert_eq!(
            mask_only(ours, &[], &[]).unwrap(),
            mask_only(cxf, &[], &[]).unwrap()
        );
    }
}
