//! Layer-1 replay/diff harness.
//!
//! Loads every `scenarios/*.toml`, replays each through the controlled SUT
//! (or via the pure discovery helpers for `service="discovery"` scenarios),
//! asserts status + invariants, then diffs the masked response against a frozen
//! snapshot.  Set `CROSSREF_REGEN=1` to capture new baselines.

use std::collections::HashMap;

use onvif_crossref::{
    fixture::FIXTURE_SCOPES,
    invariants::{check as check_invariant, InvariantCtx},
    masks::resolve_all,
    normalize::mask_only,
    scenario::{AuthMode, Scenario, Transport},
    snapshot::SnapshotStore,
    sut::{build_controlled_sut, inject_wsse, service_path},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scenarios_dir() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is set by cargo test to the crate root.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    std::path::PathBuf::from(manifest).join("scenarios")
}

fn snapshots_dir() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    std::path::PathBuf::from(manifest).join("snapshots")
}

fn regen() -> bool {
    std::env::var("CROSSREF_REGEN")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Load sorted scenario TOML paths.
fn load_scenario_paths() -> Vec<std::path::PathBuf> {
    let dir = scenarios_dir();
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read scenarios dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    paths.sort();
    paths
}

/// Read a request fixture file relative to the scenarios dir.
fn read_request_file(request_file: &str) -> Vec<u8> {
    let path = scenarios_dir().join(request_file);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("cannot read request file {}: {e}", path.display()))
}

/// Apply `{{name}}` substitutions from the capture map.
fn apply_substitutions(body: &[u8], captures: &HashMap<String, String>) -> Vec<u8> {
    if captures.is_empty() {
        return body.to_vec();
    }
    let mut s = String::from_utf8_lossy(body).into_owned();
    for (k, v) in captures {
        s = s.replace(&format!("{{{{{}}}}}", k), v);
    }
    s.into_bytes()
}

/// Extract the text of the first element matching `local` from `xml`.
fn extract_text(xml: &[u8], local: &str) -> Option<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut inside = false;
    let mut text = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                let raw = name.as_ref();
                let loc = match raw.iter().rposition(|&b| b == b':') {
                    Some(i) => &raw[i + 1..],
                    None => raw,
                };
                if loc == local.as_bytes() {
                    inside = true;
                    text.clear();
                }
            }
            Ok(Event::Text(ref t)) if inside => {
                text.push_str(&String::from_utf8_lossy(t.as_ref()));
            }
            Ok(Event::End(ref e)) if inside => {
                let name = e.name();
                let raw = name.as_ref();
                let loc = match raw.iter().rposition(|&b| b == b':') {
                    Some(i) => &raw[i + 1..],
                    None => raw,
                };
                if loc == local.as_bytes() {
                    return Some(text);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// Extract a value from a response according to a `capture.path`.
///
/// Paths are bare local-names (`body:ElementName` or just `ElementName`).
fn capture_value(response: &[u8], path: &str) -> Option<String> {
    // Strip leading "body:" prefix if present.
    let local = path.strip_prefix("body:").unwrap_or(path);
    extract_text(response, local)
}

/// Run invariants on RAW response bytes, panic with scenario context on failure.
fn assert_invariants(name: &str, invariants: &[String], raw: &[u8], ctx: &InvariantCtx) {
    for inv in invariants {
        check_invariant(inv, raw, ctx)
            .unwrap_or_else(|e| panic!("scenario {name}: invariant '{inv}' failed: {e}"));
    }
}

/// Normalize response, then diff vs snapshot or regen.
fn assert_snapshot(store: &SnapshotStore, snap_name: &str, raw: &[u8], masks: &[String]) {
    let (text_rules, attr_rules) = resolve_all(masks);
    let normalized_bytes = mask_only(raw, &text_rules, &attr_rules)
        .unwrap_or_else(|e| panic!("mask_only failed for snapshot '{snap_name}': {e}"));
    let normalized = String::from_utf8_lossy(&normalized_bytes).into_owned();

    if regen() || store.read(snap_name).is_none() {
        store
            .write_unverified(snap_name, &normalized)
            .unwrap_or_else(|e| panic!("write_unverified failed for '{snap_name}': {e}"));
        println!("[REGEN] snapshot captured: {snap_name}");
    } else {
        let frozen = store.read(snap_name).unwrap_or_else(|| {
            panic!("snapshot '{snap_name}' missing — run with CROSSREF_REGEN=1")
        });
        similar_asserts::assert_eq!(frozen, normalized, "snapshot mismatch for '{snap_name}'");
    }
}

// ---------------------------------------------------------------------------
// Single InvariantCtx used for HTTP scenarios (discovery fields unused).
// ---------------------------------------------------------------------------

fn default_ctx() -> InvariantCtx {
    InvariantCtx {
        request_message_id: String::new(),
        expected_endpoint: String::new(),
        expected_scopes: FIXTURE_SCOPES.iter().map(|s| s.to_string()).collect(),
    }
}

// ---------------------------------------------------------------------------
// Main test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn replay_all_scenarios() {
    let store = SnapshotStore::new(snapshots_dir());
    let sut = build_controlled_sut();
    let paths = load_scenario_paths();

    assert!(
        !paths.is_empty(),
        "no scenario .toml files found in scenarios/"
    );

    // nonce seed counter: increment for every authenticated POST so each request
    // in the run gets a unique nonce.
    let mut nonce_seed: u64 = 0;

    for path in &paths {
        let toml_str = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let scenario = Scenario::from_toml_str(&toml_str)
            .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()));

        let name = &scenario.name;

        // ── Discovery (non-HTTP) ─────────────────────────────────────────────
        if scenario.transport == Transport::UdpDiscovery || scenario.is_discovery() {
            run_discovery_scenario(&store, &scenario, name);
            continue;
        }

        // ── HTTP scenarios ───────────────────────────────────────────────────
        if scenario.is_multistep() {
            run_steps_scenario(&sut, &store, &scenario, name, &mut nonce_seed).await;
        } else {
            run_single_scenario(&sut, &store, &scenario, name, &mut nonce_seed).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Single-request HTTP scenario
// ---------------------------------------------------------------------------

async fn run_single_scenario(
    sut: &onvif_crossref::sut::Sut,
    store: &SnapshotStore,
    scenario: &Scenario,
    name: &str,
    nonce_seed: &mut u64,
) {
    let request_file = scenario
        .request_file
        .as_deref()
        .unwrap_or_else(|| panic!("scenario {name}: missing request_file"));
    let raw_request = read_request_file(request_file);

    // Inject WS-Security if required.
    let body = if scenario.auth_mode() == AuthMode::Usernametoken {
        let seed = *nonce_seed;
        *nonce_seed += 1;
        inject_wsse(&raw_request, seed)
    } else {
        raw_request
    };

    let path = service_path(&scenario.service);
    let resp = sut
        .post(path, &body, "application/soap+xml; charset=utf-8")
        .await;

    let expected_status = scenario
        .expected_status
        .unwrap_or_else(|| panic!("scenario {name}: missing expected_status"));
    assert_eq!(
        resp.status,
        expected_status,
        "scenario {name}: status mismatch (body: {})",
        resp.text()
    );

    let ctx = default_ctx();
    assert_invariants(name, &scenario.invariants, &resp.body, &ctx);
    assert_snapshot(store, name, &resp.body, &scenario.masks);
}

// ---------------------------------------------------------------------------
// Multi-step HTTP scenario
// ---------------------------------------------------------------------------

async fn run_steps_scenario(
    sut: &onvif_crossref::sut::Sut,
    store: &SnapshotStore,
    scenario: &Scenario,
    name: &str,
    nonce_seed: &mut u64,
) {
    let mut captures: HashMap<String, String> = HashMap::new();

    for (step_idx, step) in scenario.steps.iter().enumerate() {
        // Enforce inject directives (R7): a wrong/unsatisfiable inject must fail loudly
        // rather than be silently ignored by the {{name}} substitution.
        for inj in &step.inject {
            assert!(
                captures.contains_key(&inj.name),
                "scenario {name} step {step_idx}: inject '{}' references a value never captured",
                inj.name
            );
            assert!(
                inj.into.starts_with("header:") || inj.into.starts_with("body:"),
                "scenario {name} step {step_idx}: inject '{}' has unrecognized target '{}'",
                inj.name,
                inj.into
            );
        }

        let raw_request = read_request_file(&step.request_file);

        // Apply captures from prior steps.
        let substituted = apply_substitutions(&raw_request, &captures);

        // Inject WS-Security if required (step-level auth_mode not in model;
        // use scenario-level auth_mode for all steps).
        let body = if scenario.auth_mode() == AuthMode::Usernametoken {
            let seed = *nonce_seed;
            *nonce_seed += 1;
            inject_wsse(&substituted, seed)
        } else {
            substituted
        };

        let path = service_path(&scenario.service);
        let resp = sut
            .post(path, &body, "application/soap+xml; charset=utf-8")
            .await;

        assert_eq!(
            resp.status,
            step.expected_status,
            "scenario {name} step {step_idx}: status mismatch (body: {})",
            resp.text()
        );

        let ctx = default_ctx();
        assert_invariants(name, &step.invariants, &resp.body, &ctx);

        // Snapshot name: <scenario>.<stepN>
        let snap_name = format!("{name}.step{step_idx}");
        assert_snapshot(store, &snap_name, &resp.body, &step.masks);

        // Capture declared values.
        for cap in &step.capture {
            if let Some(val) = capture_value(&resp.body, &cap.path) {
                captures.insert(cap.name.clone(), val);
            } else {
                panic!(
                    "scenario {name} step {step_idx}: capture '{}' path '{}' not found in response",
                    cap.name, cap.path
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Discovery scenario (non-HTTP)
// ---------------------------------------------------------------------------

fn run_discovery_scenario(store: &SnapshotStore, scenario: &Scenario, name: &str) {
    use onvif_server::{discovery_build_probe_match, discovery_is_probe};

    let request_file = scenario
        .request_file
        .as_deref()
        .unwrap_or_else(|| panic!("discovery scenario {name}: missing request_file"));
    let probe_bytes = read_request_file(request_file);

    // Extract MessageID from the probe for RelatesTo / invariants.
    let message_id = extract_text(&probe_bytes, "MessageID").unwrap_or_default();

    // --- Positive case: well-formed Probe ---
    assert!(
        discovery_is_probe(&probe_bytes),
        "scenario {name}: discovery_is_probe returned false for the probe fixture"
    );

    // Build ProbeMatch using the SAME pinned, stable device UUID the controlled server
    // advertises (CONTROLLED_DISCOVERY_UUID). This is deterministic and is then ASSERTED by
    // the `stable_endpoint_uuid` invariant — no per-run random value, no masking.
    let xaddr = "http://controlled-onvif:8080/onvif/device_service";
    let device_uuid = onvif_crossref::fixture::CONTROLLED_DISCOVERY_UUID;
    let probe_match_xml = discovery_build_probe_match(&message_id, xaddr, device_uuid);
    let probe_match_bytes = probe_match_xml.as_bytes();

    // Run declared invariants on the ProbeMatch.
    let ctx = InvariantCtx {
        request_message_id: message_id.clone(),
        expected_endpoint: format!("urn:uuid:{device_uuid}"),
        // ProbeMatch advertises the discovery type-scope, not the GetScopes list.
        expected_scopes: onvif_crossref::fixture::FIXTURE_DISCOVERY_SCOPES
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };
    assert_invariants(name, &scenario.invariants, probe_match_bytes, &ctx);

    // Snapshot with wsa_message_id masked.
    let snap_masks = if scenario.masks.is_empty() {
        vec!["wsa_message_id".to_string()]
    } else {
        scenario.masks.clone()
    };
    assert_snapshot(store, name, probe_match_bytes, &snap_masks);

    // --- Negative case: non-Probe → discovery_is_probe == false ---
    // We use a minimal non-Probe XML envelope.
    let non_probe = br#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><NotAProbe/></s:Body></s:Envelope>"#;
    assert!(
        !discovery_is_probe(non_probe),
        "scenario {name}: discovery_is_probe must return false for non-Probe messages"
    );
}
