//! ONVIF response projections for `srvd_projection` mode (spec §6).
//!
//! Each extractor parses a SOAP response body and returns a `CanonProjection` —
//! a canonical, order-independent, comparable structure.  XAddr fields store
//! only the URL PATH (scheme + host:port authority stripped) so devices on
//! different hosts compare equal.  Parsing is namespace-prefix-agnostic: we
//! compare by local name only.

use std::collections::BTreeMap;

use quick_xml::events::Event;
use quick_xml::Reader;

// ─── Public types ─────────────────────────────────────────────────────────────

/// A single projected entry.  Holds the fields we care about for the given
/// operation.  Extra fields in the real response are intentionally not stored.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjEntry {
    /// URL path component of the XAddr (authority stripped).
    pub xaddr_path: Option<String>,
    /// Version Major (if present).
    pub version_major: Option<String>,
    /// Version Minor (if present).
    pub version_minor: Option<String>,
    /// Arbitrary named boolean / string fields (e.g. Events capability flags).
    pub fields: BTreeMap<String, String>,
}

/// Canonical, order-independent projection.  Keyed by:
/// - `get_capabilities`: category local-name (Device / Media / PTZ / Imaging / Events)
/// - `get_services`:     service Namespace URI
/// - `get_profiles`:     Profile token attribute
pub type CanonProjection = BTreeMap<String, ProjEntry>;

// ─── URL path extraction ───────────────────────────────────────────────────────

/// Strip scheme + authority from a URL, returning the path (and query/fragment).
/// If parsing fails we return the original string unchanged so nothing silently
/// disappears.
fn url_path(url: &str) -> String {
    // Find "://" then skip to the next "/" after the authority.
    if let Some(after_scheme) = url.split_once("://") {
        if let Some(slash) = after_scheme.1.find('/') {
            return after_scheme.1[slash..].to_string();
        }
        return "/".to_string();
    }
    url.to_string()
}

// ─── XML helpers ──────────────────────────────────────────────────────────────

/// Return just the local name (strip namespace prefix).
fn local_name(name: &[u8]) -> &[u8] {
    if let Some(pos) = name.iter().position(|&b| b == b':') {
        &name[pos + 1..]
    } else {
        name
    }
}

fn local_name_str(name: &[u8]) -> String {
    String::from_utf8_lossy(local_name(name)).into_owned()
}

fn attr_value(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        if local_name_str(attr.key.as_ref()) == name {
            return Some(String::from_utf8_lossy(&attr.value).into_owned());
        }
    }
    None
}

// ─── GetCapabilities projection ──────────────────────────────────────────────

/// Project a `GetCapabilitiesResponse` body.
///
/// Projection = set of advertised service categories (Device / Media / PTZ /
/// Imaging / Events) each with its XAddr PATH and, for Events, the
/// `WSSubscriptionPolicySupport` and `WSPullPointSupport` booleans.
pub fn get_capabilities(response: &[u8]) -> Result<CanonProjection, String> {
    let mut reader = Reader::from_reader(response);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut proj = CanonProjection::new();

    let mut current_category: Option<String> = None;
    let mut current_entry: Option<ProjEntry> = None;
    let mut inside_xaddr = false;
    let mut inside_ws_sub_policy = false;
    let mut inside_ws_pull = false;

    let known_categories = ["Device", "Media", "PTZ", "Imaging", "Events"];

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(format!("xml parse: {e}")),
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                let lname = local_name_str(e.name().as_ref());
                if known_categories.contains(&lname.as_str()) {
                    current_category = Some(lname);
                    current_entry = Some(ProjEntry::default());
                } else if current_category.is_some() {
                    match lname.as_str() {
                        "XAddr" => inside_xaddr = true,
                        "WSSubscriptionPolicySupport" => inside_ws_sub_policy = true,
                        "WSPullPointSupport" => inside_ws_pull = true,
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let lname = local_name_str(e.name().as_ref());
                if known_categories.contains(&lname.as_str()) {
                    if let (Some(cat), Some(entry)) =
                        (current_category.take(), current_entry.take())
                    {
                        proj.insert(cat, entry);
                    }
                }
                match lname.as_str() {
                    "XAddr" => inside_xaddr = false,
                    "WSSubscriptionPolicySupport" => inside_ws_sub_policy = false,
                    "WSPullPointSupport" => inside_ws_pull = false,
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                let text_val = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                if text_val.is_empty() {
                    buf.clear();
                    continue;
                }
                if let Some(ref mut entry) = current_entry {
                    if inside_xaddr {
                        entry.xaddr_path = Some(url_path(&text_val));
                    } else if inside_ws_sub_policy {
                        entry
                            .fields
                            .insert("WSSubscriptionPolicySupport".to_string(), text_val);
                    } else if inside_ws_pull {
                        entry
                            .fields
                            .insert("WSPullPointSupport".to_string(), text_val);
                    }
                }
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(proj)
}

// ─── GetServices projection ───────────────────────────────────────────────────

/// Project a `GetServicesResponse` body.
///
/// Projection = set of service Namespace values, each with XAddr PATH and
/// Version Major/Minor.
pub fn get_services(response: &[u8]) -> Result<CanonProjection, String> {
    let mut reader = Reader::from_reader(response);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut proj = CanonProjection::new();

    let mut in_service = false;
    let mut current_ns: Option<String> = None;
    let mut current_entry: Option<ProjEntry> = None;
    let mut inside_namespace = false;
    let mut inside_xaddr = false;
    let mut inside_major = false;
    let mut inside_minor = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(format!("xml parse: {e}")),
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                let lname = local_name_str(e.name().as_ref());
                match lname.as_str() {
                    "Service" => {
                        in_service = true;
                        current_ns = None;
                        current_entry = Some(ProjEntry::default());
                    }
                    "Namespace" if in_service => inside_namespace = true,
                    "XAddr" if in_service => inside_xaddr = true,
                    "Major" if in_service => inside_major = true,
                    "Minor" if in_service => inside_minor = true,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let lname = local_name_str(e.name().as_ref());
                if lname == "Service" {
                    if let (Some(ns), Some(entry)) = (current_ns.take(), current_entry.take()) {
                        proj.insert(ns, entry);
                    }
                    in_service = false;
                }
                match lname.as_str() {
                    "Namespace" => inside_namespace = false,
                    "XAddr" => inside_xaddr = false,
                    "Major" => inside_major = false,
                    "Minor" => inside_minor = false,
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                if !in_service {
                    buf.clear();
                    continue;
                }
                let text_val = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                if text_val.is_empty() {
                    buf.clear();
                    continue;
                }
                if inside_namespace {
                    current_ns = Some(text_val);
                } else if let Some(ref mut entry) = current_entry {
                    if inside_xaddr {
                        entry.xaddr_path = Some(url_path(&text_val));
                    } else if inside_major {
                        entry.version_major = Some(text_val);
                    } else if inside_minor {
                        entry.version_minor = Some(text_val);
                    }
                }
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(proj)
}

// ─── GetProfiles projection ───────────────────────────────────────────────────

/// Project a `GetProfilesResponse` body.
///
/// Projection = token-pinned set of profiles.  Each entry holds the profile
/// Name and the config token attributes (VideoSourceConfiguration,
/// VideoEncoderConfiguration, PTZConfiguration).  Enough to detect our-side
/// divergence without deep field comparison.
pub fn get_profiles(response: &[u8]) -> Result<CanonProjection, String> {
    let mut reader = Reader::from_reader(response);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut proj = CanonProjection::new();

    let mut current_token: Option<String> = None;
    let mut current_entry: Option<ProjEntry> = None;
    let mut depth_in_profile: u32 = 0;
    let mut inside_profile_name = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(format!("xml parse: {e}")),
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) => {
                let lname = local_name_str(e.name().as_ref());
                if lname == "Profiles" && current_token.is_none() {
                    let token = attr_value(e, "token").unwrap_or_default();
                    current_token = Some(token);
                    current_entry = Some(ProjEntry::default());
                    depth_in_profile = 1;
                    inside_profile_name = false;
                } else if depth_in_profile > 0 {
                    depth_in_profile += 1;
                    match lname.as_str() {
                        // Depth 2 = direct children of Profiles
                        "Name" if depth_in_profile == 2 => inside_profile_name = true,
                        "VideoSourceConfiguration" => {
                            let tok = attr_value(e, "token").unwrap_or_default();
                            if let Some(ref mut entry) = current_entry {
                                entry
                                    .fields
                                    .insert("VideoSourceConfiguration.token".to_string(), tok);
                            }
                        }
                        "VideoEncoderConfiguration" => {
                            let tok = attr_value(e, "token").unwrap_or_default();
                            if let Some(ref mut entry) = current_entry {
                                entry
                                    .fields
                                    .insert("VideoEncoderConfiguration.token".to_string(), tok);
                            }
                        }
                        "PTZConfiguration" => {
                            let tok = attr_value(e, "token").unwrap_or_default();
                            if let Some(ref mut entry) = current_entry {
                                entry
                                    .fields
                                    .insert("PTZConfiguration.token".to_string(), tok);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let lname = local_name_str(e.name().as_ref());
                if lname == "Profiles" && depth_in_profile > 0 {
                    if let (Some(token), Some(entry)) = (current_token.take(), current_entry.take())
                    {
                        proj.insert(token, entry);
                    }
                    depth_in_profile = 0;
                    inside_profile_name = false;
                } else if depth_in_profile > 0 {
                    if lname == "Name" {
                        inside_profile_name = false;
                    }
                    depth_in_profile -= 1;
                }
            }
            Ok(Event::Text(ref t)) => {
                if inside_profile_name {
                    let text_val = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                    if !text_val.is_empty() {
                        if let Some(ref mut entry) = current_entry {
                            entry.fields.insert("Name".to_string(), text_val);
                        }
                    }
                }
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(proj)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer2::verdict::{evaluate_none, evaluate_projection, Verdict};

    // ── sample XML fixtures ───────────────────────────────────────────────────

    fn sample_capabilities() -> &'static [u8] {
        include_bytes!("../snapshots/device_get_capabilities.xml")
    }

    fn sample_services() -> &'static [u8] {
        include_bytes!("../snapshots/device_get_services.xml")
    }

    fn sample_profiles() -> &'static [u8] {
        include_bytes!("../snapshots/media_get_profiles.xml")
    }

    // ── get_capabilities ────────────────────────────────────────────────────

    #[test]
    fn get_capabilities_extracts_all_categories() {
        let proj = get_capabilities(sample_capabilities()).unwrap();
        for cat in &["Device", "Media", "PTZ", "Imaging", "Events"] {
            assert!(proj.contains_key(*cat), "missing category: {cat}");
        }
        assert_eq!(proj.len(), 5);
    }

    #[test]
    fn get_capabilities_xaddr_paths_stripped() {
        let proj = get_capabilities(sample_capabilities()).unwrap();
        for (cat, entry) in &proj {
            let path = entry.xaddr_path.as_deref().expect("xaddr_path present");
            assert!(
                path.starts_with('/'),
                "category {cat}: path should start with /: {path}"
            );
            assert!(
                !path.contains("://"),
                "category {cat}: authority not stripped: {path}"
            );
        }
    }

    #[test]
    fn get_capabilities_events_booleans_captured() {
        let proj = get_capabilities(sample_capabilities()).unwrap();
        let events = proj.get("Events").expect("Events present");
        assert_eq!(
            events
                .fields
                .get("WSSubscriptionPolicySupport")
                .map(|s| s.as_str()),
            Some("false"),
            "WSSubscriptionPolicySupport"
        );
        assert_eq!(
            events.fields.get("WSPullPointSupport").map(|s| s.as_str()),
            Some("true"),
            "WSPullPointSupport"
        );
    }

    #[test]
    fn get_capabilities_non_events_have_no_ws_booleans() {
        let proj = get_capabilities(sample_capabilities()).unwrap();
        for cat in &["Device", "Media", "PTZ", "Imaging"] {
            let entry = proj.get(*cat).unwrap();
            assert!(
                !entry.fields.contains_key("WSSubscriptionPolicySupport"),
                "category {cat} should not have WSSubscriptionPolicySupport"
            );
        }
    }

    // ── get_services ────────────────────────────────────────────────────────

    #[test]
    fn get_services_extracts_five_services() {
        let proj = get_services(sample_services()).unwrap();
        assert_eq!(proj.len(), 5);
    }

    #[test]
    fn get_services_namespaces_present() {
        let proj = get_services(sample_services()).unwrap();
        let expected_ns = [
            "http://www.onvif.org/ver10/device/wsdl",
            "http://www.onvif.org/ver10/media/wsdl",
            "http://www.onvif.org/ver10/ptz/wsdl",
            "http://www.onvif.org/ver20/imaging/wsdl",
            "http://www.onvif.org/ver10/events/wsdl",
        ];
        for ns in &expected_ns {
            assert!(proj.contains_key(*ns), "missing namespace: {ns}");
        }
    }

    #[test]
    fn get_services_xaddr_paths_stripped() {
        let proj = get_services(sample_services()).unwrap();
        for (ns, entry) in &proj {
            let path = entry.xaddr_path.as_deref().expect("xaddr_path present");
            assert!(
                path.starts_with('/'),
                "ns {ns}: path should start with /: {path}"
            );
            assert!(
                !path.contains("://"),
                "ns {ns}: authority not stripped: {path}"
            );
        }
    }

    #[test]
    fn get_services_version_extracted() {
        let proj = get_services(sample_services()).unwrap();
        let dev = proj.get("http://www.onvif.org/ver10/device/wsdl").unwrap();
        assert_eq!(dev.version_major.as_deref(), Some("2"));
        assert_eq!(dev.version_minor.as_deref(), Some("42"));
    }

    // ── get_profiles ────────────────────────────────────────────────────────

    #[test]
    fn get_profiles_extracts_profile_token() {
        let proj = get_profiles(sample_profiles()).unwrap();
        assert_eq!(proj.len(), 1);
        assert!(proj.contains_key("profile_0"));
    }

    #[test]
    fn get_profiles_name_captured() {
        let proj = get_profiles(sample_profiles()).unwrap();
        let p = proj.get("profile_0").unwrap();
        assert_eq!(
            p.fields.get("Name").map(|s| s.as_str()),
            Some("ControlledProfile")
        );
    }

    #[test]
    fn get_profiles_config_tokens_captured() {
        let proj = get_profiles(sample_profiles()).unwrap();
        let p = proj.get("profile_0").unwrap();
        assert_eq!(
            p.fields
                .get("VideoSourceConfiguration.token")
                .map(|s| s.as_str()),
            Some("video_src_0")
        );
        assert_eq!(
            p.fields
                .get("VideoEncoderConfiguration.token")
                .map(|s| s.as_str()),
            Some("video_enc_0")
        );
        assert_eq!(
            p.fields.get("PTZConfiguration.token").map(|s| s.as_str()),
            Some("ptz_cfg_0")
        );
    }

    // ── evaluate_projection — srvd extra ignored (asymmetric rule) ───────────

    /// srvd has an extra service beyond ours; evaluate_projection should Pass.
    #[test]
    fn evaluate_projection_srvd_extra_service_is_pass() {
        // "ours" = fixture = 2 services
        let our = get_services(sample_services()).unwrap();
        let fixture = our.clone();

        // srvd has those 5 PLUS an extra one
        let mut srvd = our.clone();
        srvd.insert(
            "http://www.example.com/extra/wsdl".to_string(),
            ProjEntry {
                xaddr_path: Some("/onvif/extra".to_string()),
                version_major: Some("1".to_string()),
                version_minor: Some("0".to_string()),
                fields: BTreeMap::new(),
            },
        );

        assert_eq!(evaluate_projection(&fixture, &our, &srvd), Verdict::Pass);
    }

    // ── evaluate_projection — our-side extra → SutFail ───────────────────────

    /// our server advertises a service not in the fixture → SutFail.
    #[test]
    fn evaluate_projection_our_side_extra_is_sut_fail() {
        let fixture = get_services(sample_services()).unwrap();

        let mut our = fixture.clone();
        our.insert(
            "http://www.example.com/extra/wsdl".to_string(),
            ProjEntry {
                xaddr_path: Some("/onvif/extra".to_string()),
                version_major: Some("1".to_string()),
                version_minor: Some("0".to_string()),
                fields: BTreeMap::new(),
            },
        );

        let srvd = fixture.clone(); // srvd matches fixture exactly

        assert!(matches!(
            evaluate_projection(&fixture, &our, &srvd),
            Verdict::SutFail(_)
        ));
    }

    // ── evaluate_projection — srvd missing entry we advertise ────────────────

    /// srvd is missing a service we (and the fixture) advertise → ReferenceDisagreement.
    #[test]
    fn evaluate_projection_srvd_missing_our_entry_is_reference_disagreement() {
        let fixture = get_services(sample_services()).unwrap();
        let our = fixture.clone();

        // srvd is missing one of our services
        let mut srvd = our.clone();
        srvd.remove("http://www.onvif.org/ver10/events/wsdl");

        assert!(matches!(
            evaluate_projection(&fixture, &our, &srvd),
            Verdict::ReferenceDisagreement(_)
        ));
    }

    // ── evaluate_projection — srvd entry differs in a field ──────────────────

    /// srvd has a service we advertise but with a different xaddr_path → ReferenceDisagreement.
    #[test]
    fn evaluate_projection_srvd_differing_field_is_reference_disagreement() {
        let fixture = get_services(sample_services()).unwrap();
        let our = fixture.clone();

        let mut srvd = our.clone();
        // Mutate the events service xaddr_path
        srvd.get_mut("http://www.onvif.org/ver10/events/wsdl")
            .unwrap()
            .xaddr_path = Some("/different/path".to_string());

        assert!(matches!(
            evaluate_projection(&fixture, &our, &srvd),
            Verdict::ReferenceDisagreement(_)
        ));
    }

    // ── evaluate_projection — exact match → Pass ─────────────────────────────

    #[test]
    fn evaluate_projection_identical_is_pass() {
        let proj = get_services(sample_services()).unwrap();
        assert_eq!(evaluate_projection(&proj, &proj, &proj), Verdict::Pass);
    }

    // ── evaluate_none ────────────────────────────────────────────────────────

    #[test]
    fn evaluate_none_schema_invalid_is_sut_fail() {
        assert!(matches!(
            evaluate_none(true, false, true),
            Verdict::SutFail(_)
        ));
    }

    #[test]
    fn evaluate_none_outcome_mismatch_is_sut_fail() {
        // declared success but our server returned fault
        assert!(matches!(
            evaluate_none(true, true, false),
            Verdict::SutFail(_)
        ));
    }

    #[test]
    fn evaluate_none_declared_success_our_success_schema_valid_is_pass() {
        assert_eq!(evaluate_none(true, true, true), Verdict::Pass);
    }

    #[test]
    fn evaluate_none_declared_fault_our_fault_schema_valid_is_pass() {
        assert_eq!(evaluate_none(false, true, false), Verdict::Pass);
    }

    // ── malformed-XML error propagation ──────────────────────────────────────

    #[test]
    fn get_capabilities_malformed_xml_returns_err() {
        assert!(get_capabilities(b"not xml <<").is_err());
    }

    #[test]
    fn get_services_malformed_xml_returns_err() {
        assert!(get_services(b"not xml <<").is_err());
    }

    #[test]
    fn get_profiles_malformed_xml_returns_err() {
        assert!(get_profiles(b"not xml <<").is_err());
    }
}
