//! Named structural-invariant registry (spec §5.2).
//!
//! Each invariant is a named assertion over RAW response bytes (before masking).
//! Parsing is local-name-based via quick-xml so namespace-prefix variations are
//! transparent.
//!
//! ## Invariant catalogue
//!
//! | Name                        | What it asserts                                            |
//! |-----------------------------|------------------------------------------------------------|
//! | `single_white_balance`      | Exactly one element with local-name `WhiteBalance`         |
//! | `ptz_move_status_attr`      | An element `Capabilities` has attribute `MoveStatus`=`true`|
//! | `relates_to_matches_probe`  | `RelatesTo` text == `ctx.request_message_id`               |
//! | `stable_endpoint_uuid`      | Response contains `ctx.expected_endpoint` as a substring   |
//! | `scopes_match_fixture`      | Every scope in `ctx.expected_scopes` appears; counts match |
//! | `xaddrs_escaped`            | Every `XAddrs`/`XAddr` element text parses as valid XML    |
//! | `wsa_subscription_id_present` | A `SubscriptionId` element is present                    |

use quick_xml::events::Event;
use quick_xml::Reader;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Context values injected by the harness for invariants that need to compare
/// against scenario-specific expected values.
pub struct InvariantCtx {
    /// The WS-Addressing MessageID from the Probe request (for
    /// `relates_to_matches_probe`).
    pub request_message_id: String,
    /// The stable endpoint UUID the fixture is configured to emit (for
    /// `stable_endpoint_uuid`).
    pub expected_endpoint: String,
    /// The full list of scope strings the fixture is configured to emit (for
    /// `scopes_match_fixture`).
    pub expected_scopes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the named invariant against `response` raw bytes.
///
/// Returns `Ok(())` when the response satisfies the invariant, or
/// `Err(reason)` describing the violation.  Unknown names return
/// `Err("unknown invariant: …")`.
pub fn check(name: &str, response: &[u8], ctx: &InvariantCtx) -> Result<(), String> {
    match name {
        "single_white_balance" => exactly_one_element(response, "WhiteBalance"),
        "ptz_move_status_attr" => attr_equals(response, "Capabilities", "MoveStatus", "true"),
        "relates_to_matches_probe" => {
            text_of_element_equals(response, "RelatesTo", &ctx.request_message_id)
        }
        "stable_endpoint_uuid" => bytes_contain(response, &ctx.expected_endpoint),
        "scopes_match_fixture" => scopes_match(response, &ctx.expected_scopes),
        "xaddrs_escaped" => xaddrs_well_formed(response),
        "wsa_subscription_id_present" => element_present(response, "SubscriptionId"),
        other => Err(format!("unknown invariant: {other}")),
    }
}

// ---------------------------------------------------------------------------
// Helpers — all local-name based, prefix-agnostic
// ---------------------------------------------------------------------------

/// Return the local name part of a qualified name (everything after the last `:`).
fn local_name(qname: &[u8]) -> &[u8] {
    match qname.iter().rposition(|&b| b == b':') {
        Some(i) => &qname[i + 1..],
        None => qname,
    }
}

/// Count elements with the given local name in `xml`.
fn count_elements(xml: &[u8], target_local: &str) -> Result<usize, String> {
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut count = 0usize;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if local_name(e.name().as_ref()) == target_local.as_bytes() {
                    count += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
        buf.clear();
    }
    Ok(count)
}

/// Assert exactly one element with the given local name exists.
fn exactly_one_element(xml: &[u8], local: &str) -> Result<(), String> {
    let count = count_elements(xml, local)?;
    if count == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected exactly 1 <{local}> element, found {count}"
        ))
    }
}

/// Assert at least one element with the given local name exists.
fn element_present(xml: &[u8], local: &str) -> Result<(), String> {
    let count = count_elements(xml, local)?;
    if count >= 1 {
        Ok(())
    } else {
        Err(format!(
            "expected at least one <{local}> element, found none"
        ))
    }
}

/// Assert that some element with `elem_local` name has an attribute `attr_local`
/// whose value equals `expected_val`.
fn attr_equals(
    xml: &[u8],
    elem_local: &str,
    attr_local: &str,
    expected_val: &str,
) -> Result<(), String> {
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut found_elem = false;
    let mut found_attr = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                if local_name(e.name().as_ref()) == elem_local.as_bytes() {
                    found_elem = true;
                    for attr_res in e.attributes() {
                        let attr = attr_res.map_err(|err| format!("attr error: {err}"))?;
                        if local_name(attr.key.as_ref()) == attr_local.as_bytes() {
                            let val =
                                std::str::from_utf8(&attr.value).map_err(|e| e.to_string())?;
                            if val == expected_val {
                                found_attr = true;
                            } else {
                                return Err(format!(
                                    "<{elem_local} {attr_local}=\"...\">: expected \"{expected_val}\", got \"{val}\""
                                ));
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
        buf.clear();
    }
    if !found_elem {
        return Err(format!("no <{elem_local}> element found in response"));
    }
    if !found_attr {
        return Err(format!(
            "<{elem_local}> found but attribute \"{attr_local}\" is absent"
        ));
    }
    Ok(())
}

/// Assert the text content of the first element matching `elem_local` equals `expected`.
fn text_of_element_equals(xml: &[u8], elem_local: &str, expected: &str) -> Result<(), String> {
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut inside = false;
    let mut collected = String::new();
    let mut found = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if local_name(e.name().as_ref()) == elem_local.as_bytes() {
                    inside = true;
                    collected.clear();
                    found = true;
                }
            }
            Ok(Event::Text(ref t)) if inside => {
                let s = String::from_utf8_lossy(t.as_ref()).into_owned();
                collected.push_str(&s);
            }
            Ok(Event::End(ref e)) if inside => {
                if local_name(e.name().as_ref()) == elem_local.as_bytes() {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
        buf.clear();
    }
    if !found {
        return Err(format!("no <{elem_local}> element found in response"));
    }
    if collected == expected {
        Ok(())
    } else {
        Err(format!(
            "<{elem_local}> text mismatch: expected {:?}, got {:?}",
            expected, collected
        ))
    }
}

/// Assert the raw bytes contain `needle` as a substring.
fn bytes_contain(xml: &[u8], needle: &str) -> Result<(), String> {
    if xml.windows(needle.len()).any(|w| w == needle.as_bytes()) {
        Ok(())
    } else {
        Err(format!(
            "response does not contain expected substring {:?}",
            needle
        ))
    }
}

/// Collect the text content of all elements with the given local name.
fn collect_element_texts(xml: &[u8], target_local: &str) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut results = Vec::new();
    let mut inside = false;
    let mut current = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if local_name(e.name().as_ref()) == target_local.as_bytes() {
                    inside = true;
                    current.clear();
                }
            }
            Ok(Event::Text(ref t)) if inside => {
                current.push_str(&String::from_utf8_lossy(t.as_ref()));
            }
            Ok(Event::End(ref e)) if inside => {
                if local_name(e.name().as_ref()) == target_local.as_bytes() {
                    inside = false;
                    results.push(current.clone());
                    current.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
        buf.clear();
    }
    Ok(results)
}

/// Assert every scope in `expected` appears in the response `Scope` elements,
/// and the total count matches.
fn scopes_match(xml: &[u8], expected: &[String]) -> Result<(), String> {
    let actual = collect_element_texts(xml, "Scope")?;
    if actual.len() != expected.len() {
        return Err(format!(
            "scope count mismatch: expected {}, got {} (actual: {:?})",
            expected.len(),
            actual.len(),
            actual
        ));
    }
    for scope in expected {
        if !actual.contains(scope) {
            return Err(format!(
                "expected scope {:?} not found in response scopes: {:?}",
                scope, actual
            ));
        }
    }
    Ok(())
}

/// Assert every `XAddrs` or `XAddr` element text is non-empty and contains no
/// raw unescaped `&` or `<` characters (quick-xml would have already failed to
/// parse the envelope if truly malformed, but we also assert presence).
fn xaddrs_well_formed(xml: &[u8]) -> Result<(), String> {
    // quick-xml already enforces well-formedness during parsing; if we got here
    // the XML was parseable.  Collect XAddrs/XAddr texts and assert non-empty.
    let mut found_any = false;
    for local in ["XAddrs", "XAddr"] {
        let texts = collect_element_texts(xml, local)?;
        for text in texts {
            found_any = true;
            if text.trim().is_empty() {
                return Err(format!("<{local}> element text is empty"));
            }
            // quick-xml would have errored on unescaped < or & during parse;
            // belt-and-suspenders: check the raw element text for '&'/'<'.
            if text.contains('&') || text.contains('<') {
                return Err(format!(
                    "<{local}> text contains raw unescaped XML special characters: {:?}",
                    text
                ));
            }
        }
    }
    if !found_any {
        return Err("no XAddrs or XAddr element found in response".to_string());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> InvariantCtx {
        InvariantCtx {
            request_message_id: "urn:uuid:test-probe-id-1234".to_string(),
            expected_endpoint: "urn:uuid:controlled-endpoint-uuid-5678".to_string(),
            expected_scopes: vec![
                "onvif://www.onvif.org/type/video_encoder".to_string(),
                "onvif://www.onvif.org/hardware/Controlled-1".to_string(),
            ],
        }
    }

    // ── single_white_balance ─────────────────────────────────────────────────

    const ONE_WB: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><timg:GetImagingSettingsResponse xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema"><timg:ImagingSettings><tt:WhiteBalance><tt:Mode>AUTO</tt:Mode></tt:WhiteBalance></timg:ImagingSettings></timg:GetImagingSettingsResponse></env:Body></env:Envelope>"#;
    const TWO_WB: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><timg:GetImagingSettingsResponse xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema"><timg:ImagingSettings><tt:WhiteBalance><tt:Mode>AUTO</tt:Mode></tt:WhiteBalance><tt:WhiteBalance><tt:Mode>MANUAL</tt:Mode></tt:WhiteBalance></timg:ImagingSettings></timg:GetImagingSettingsResponse></env:Body></env:Envelope>"#;
    const ZERO_WB: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><timg:GetImagingSettingsResponse xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl"><timg:ImagingSettings/></timg:GetImagingSettingsResponse></env:Body></env:Envelope>"#;

    #[test]
    fn single_white_balance_pass() {
        assert!(check("single_white_balance", ONE_WB, &ctx()).is_ok());
    }

    #[test]
    fn single_white_balance_fail_two() {
        let err = check("single_white_balance", TWO_WB, &ctx()).unwrap_err();
        assert!(err.contains("found 2"), "expected '2' in: {err}");
    }

    #[test]
    fn single_white_balance_fail_zero() {
        let err = check("single_white_balance", ZERO_WB, &ctx()).unwrap_err();
        assert!(err.contains("found 0"), "expected '0' in: {err}");
    }

    // ── ptz_move_status_attr ─────────────────────────────────────────────────

    const MOVE_STATUS_TRUE: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tptz:GetServiceCapabilitiesResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"><tptz:Capabilities MoveStatus="true" Snapshot="false"/></tptz:GetServiceCapabilitiesResponse></env:Body></env:Envelope>"#;
    const MOVE_STATUS_FALSE: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tptz:GetServiceCapabilitiesResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"><tptz:Capabilities MoveStatus="false" Snapshot="false"/></tptz:GetServiceCapabilitiesResponse></env:Body></env:Envelope>"#;
    const NO_CAPABILITIES: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tptz:GetServiceCapabilitiesResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/></env:Body></env:Envelope>"#;

    #[test]
    fn ptz_move_status_attr_pass() {
        assert!(check("ptz_move_status_attr", MOVE_STATUS_TRUE, &ctx()).is_ok());
    }

    #[test]
    fn ptz_move_status_attr_fail_false() {
        let err = check("ptz_move_status_attr", MOVE_STATUS_FALSE, &ctx()).unwrap_err();
        assert!(err.contains("false"), "error should mention 'false': {err}");
    }

    #[test]
    fn ptz_move_status_attr_fail_no_capabilities() {
        let err = check("ptz_move_status_attr", NO_CAPABILITIES, &ctx()).unwrap_err();
        assert!(
            err.contains("Capabilities"),
            "error should mention missing element: {err}"
        );
    }

    // ── relates_to_matches_probe ─────────────────────────────────────────────

    const RELATES_TO_MATCH: &[u8] = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Header><a:RelatesTo xmlns:a="http://www.w3.org/2005/08/addressing">urn:uuid:test-probe-id-1234</a:RelatesTo></s:Header><s:Body/></s:Envelope>"#;
    const RELATES_TO_MISMATCH: &[u8] = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Header><a:RelatesTo xmlns:a="http://www.w3.org/2005/08/addressing">urn:uuid:different-id-9999</a:RelatesTo></s:Header><s:Body/></s:Envelope>"#;

    #[test]
    fn relates_to_matches_probe_pass() {
        assert!(check("relates_to_matches_probe", RELATES_TO_MATCH, &ctx()).is_ok());
    }

    #[test]
    fn relates_to_matches_probe_fail() {
        let err = check("relates_to_matches_probe", RELATES_TO_MISMATCH, &ctx()).unwrap_err();
        assert!(
            err.contains("mismatch") || err.contains("expected"),
            "bad err: {err}"
        );
    }

    // ── stable_endpoint_uuid ─────────────────────────────────────────────────

    const HAS_ENDPOINT: &[u8] = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Header><a:EndpointReference xmlns:a="http://www.w3.org/2005/08/addressing"><a:Address>urn:uuid:controlled-endpoint-uuid-5678</a:Address></a:EndpointReference></s:Header><s:Body/></s:Envelope>"#;
    const MISSING_ENDPOINT: &[u8] = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><SomeResponse/></s:Body></s:Envelope>"#;

    #[test]
    fn stable_endpoint_uuid_pass() {
        assert!(check("stable_endpoint_uuid", HAS_ENDPOINT, &ctx()).is_ok());
    }

    #[test]
    fn stable_endpoint_uuid_fail() {
        let err = check("stable_endpoint_uuid", MISSING_ENDPOINT, &ctx()).unwrap_err();
        assert!(err.contains("substring"), "bad err: {err}");
    }

    // ── scopes_match_fixture ─────────────────────────────────────────────────

    const SCOPES_MATCH: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tds:GetScopesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl"><tds:Scopes><tt:ScopeDef>Fixed</tt:ScopeDef><tt:ScopeItem><tt:ScopeDefinition>Fixed</tt:ScopeDefinition><tds:Scope>onvif://www.onvif.org/type/video_encoder</tds:Scope></tt:ScopeItem><tt:ScopeItem><tt:ScopeDefinition>Fixed</tt:ScopeDefinition><tds:Scope>onvif://www.onvif.org/hardware/Controlled-1</tds:Scope></tt:ScopeItem></tds:Scopes></tds:GetScopesResponse></env:Body></env:Envelope>"#;
    const SCOPES_MISSING_ONE: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tds:GetScopesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl"><tds:Scopes><tds:Scope>onvif://www.onvif.org/type/video_encoder</tds:Scope></tds:Scopes></tds:GetScopesResponse></env:Body></env:Envelope>"#;

    #[test]
    fn scopes_match_fixture_pass() {
        assert!(check("scopes_match_fixture", SCOPES_MATCH, &ctx()).is_ok());
    }

    #[test]
    fn scopes_match_fixture_fail_count() {
        let err = check("scopes_match_fixture", SCOPES_MISSING_ONE, &ctx()).unwrap_err();
        assert!(
            err.contains("count") || err.contains("mismatch") || err.contains("expected 2"),
            "bad err: {err}"
        );
    }

    // ── xaddrs_escaped ───────────────────────────────────────────────────────

    const XADDRS_GOOD: &[u8] = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><d:ProbeMatch xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"><d:XAddrs>http://192.168.1.100/onvif/device_service</d:XAddrs></d:ProbeMatch></s:Body></s:Envelope>"#;
    // Raw & would break quick-xml parsing; we test a "present but empty" case instead,
    // since a raw & in the XML stream would cause a parse error before we even reach
    // our check.  The failure mode is therefore a parse error from the `collect` step.
    const XADDRS_EMPTY: &[u8] = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><d:ProbeMatch xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"><d:XAddrs></d:XAddrs></d:ProbeMatch></s:Body></s:Envelope>"#;
    const NO_XADDRS: &[u8] = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><d:ProbeMatch xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"/></s:Body></s:Envelope>"#;

    #[test]
    fn xaddrs_escaped_pass() {
        assert!(check("xaddrs_escaped", XADDRS_GOOD, &ctx()).is_ok());
    }

    #[test]
    fn xaddrs_escaped_fail_empty() {
        let err = check("xaddrs_escaped", XADDRS_EMPTY, &ctx()).unwrap_err();
        assert!(err.contains("empty"), "bad err: {err}");
    }

    #[test]
    fn xaddrs_escaped_fail_no_element() {
        let err = check("xaddrs_escaped", NO_XADDRS, &ctx()).unwrap_err();
        assert!(err.contains("no XAddrs"), "bad err: {err}");
    }

    // ── wsa_subscription_id_present ─────────────────────────────────────────

    const HAS_SUB_ID: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tev:CreatePullPointSubscriptionResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionReference><wsa5:ReferenceParameters xmlns:wsa5="http://www.w3.org/2005/08/addressing"><tev:SubscriptionId>some-uuid</tev:SubscriptionId></wsa5:ReferenceParameters></tev:SubscriptionReference></tev:CreatePullPointSubscriptionResponse></env:Body></env:Envelope>"#;
    const NO_SUB_ID: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tev:CreatePullPointSubscriptionResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/></env:Body></env:Envelope>"#;

    #[test]
    fn wsa_subscription_id_present_pass() {
        assert!(check("wsa_subscription_id_present", HAS_SUB_ID, &ctx()).is_ok());
    }

    #[test]
    fn wsa_subscription_id_present_fail() {
        let err = check("wsa_subscription_id_present", NO_SUB_ID, &ctx()).unwrap_err();
        assert!(err.contains("SubscriptionId"), "bad err: {err}");
    }

    // ── unknown invariant ────────────────────────────────────────────────────

    #[test]
    fn unknown_invariant_returns_err() {
        let err = check("no_such_invariant_xyz", b"<x/>", &ctx()).unwrap_err();
        assert!(err.contains("unknown invariant"), "bad err: {err}");
        assert!(err.contains("no_such_invariant_xyz"), "bad err: {err}");
    }
}
