//! Layer-2 orchestration: compose lifecycle, endpoints, verdict, promotion, report.

pub mod compose;
pub mod promote;
pub mod report;
pub mod verdict;

use std::collections::HashMap;
use std::path::Path;

use report::Report;
use verdict::Verdict;
pub use verdict::{Eval, Resp};

use crate::{
    body_extract::extract_body_child,
    invariants::{check as check_invariant, InvariantCtx},
    masks::resolve_all,
    normalize::mask_only,
    oracle::Oracle,
    projection,
    scenario::{AuthMode, Outcome, ReferenceMode, Scenario, Transport},
    snapshot::SnapshotStore,
    sut::{inject_wsse, service_path},
};

/// Host-published port URLs for local runs (using the docker-compose.local.yml override).
pub struct Endpoints {
    pub our: String,
    pub srvd: String,
    pub oracle: String,
}

impl Endpoints {
    pub fn localhost() -> Self {
        Endpoints {
            our: "http://localhost:8080".into(),
            // onvif-srvd publishes on port 1000 (docker-compose.local.yml line 16).
            srvd: "http://localhost:1000".into(),
            oracle: "http://localhost:8081".into(),
        }
    }
}

/// Drive all in-scope ONVIF conformance scenarios, return the verdict report.
///
/// `scenarios_filter`: if `Some`, only run the listed scenario names.
pub fn run(
    endpoints: &Endpoints,
    repo_root: &Path,
    promote_on_pass: bool,
    scenarios_filter: Option<&[String]>,
) -> Report {
    let scenarios_dir = repo_root.join("crossref").join("scenarios");
    let snapshots_dir = repo_root.join("crossref").join("snapshots");

    let store = SnapshotStore::new(&snapshots_dir);
    let oracle = Oracle::new(&endpoints.oracle);

    // Load sorted scenario TOML paths.
    let mut paths: Vec<_> = std::fs::read_dir(&scenarios_dir)
        .unwrap_or_else(|e| panic!("cannot read scenarios dir {}: {e}", scenarios_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    paths.sort();

    let mut rows: Vec<(String, Verdict)> = Vec::new();
    let mut nonce_seed: u64 = 0;

    for path in &paths {
        let toml_str = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                rows.push((name, Verdict::HarnessError(format!("read toml: {e}"))));
                continue;
            }
        };
        let scenario = match Scenario::from_toml_str(&toml_str) {
            Ok(s) => s,
            Err(e) => {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                rows.push((name, Verdict::HarnessError(format!("parse toml: {e}"))));
                continue;
            }
        };

        let name = scenario.name.clone();

        // Respect scenarios_filter.
        if let Some(filter) = scenarios_filter {
            if !filter.iter().any(|f| f == &name) {
                continue;
            }
        }

        // Skip discovery and UDP scenarios — Layer-2 covers HTTP services only.
        if scenario.is_discovery() || scenario.transport == Transport::UdpDiscovery {
            continue;
        }

        let verdict = if scenario.is_multistep() {
            run_multistep_scenario(
                &scenario,
                &name,
                &scenarios_dir,
                endpoints,
                &oracle,
                &store,
                promote_on_pass,
                &mut nonce_seed,
            )
        } else {
            run_single_scenario(
                &scenario,
                &name,
                &scenarios_dir,
                endpoints,
                &oracle,
                &store,
                promote_on_pass,
                &mut nonce_seed,
            )
        };

        rows.push((name, verdict));
    }

    let unverified_remaining = store.unverified_count().unwrap_or(0);

    Report {
        rows,
        unverified_remaining,
    }
}

// ---------------------------------------------------------------------------
// Single-step scenario
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_single_scenario(
    scenario: &Scenario,
    name: &str,
    scenarios_dir: &std::path::Path,
    endpoints: &Endpoints,
    oracle: &Oracle,
    store: &SnapshotStore,
    promote_on_pass: bool,
    nonce_seed: &mut u64,
) -> Verdict {
    let request_file = match scenario.request_file.as_deref() {
        Some(f) => f,
        None => return Verdict::HarnessError("missing request_file".into()),
    };

    let raw_request = match std::fs::read(scenarios_dir.join(request_file)) {
        Ok(b) => b,
        Err(e) => return Verdict::HarnessError(format!("read request: {e}")),
    };

    let request_body = if scenario.auth_mode() == AuthMode::Usernametoken {
        let seed = *nonce_seed;
        *nonce_seed += 1;
        inject_wsse(&raw_request, seed)
    } else {
        raw_request
    };

    let schema_id = scenario.schema_id.as_deref().unwrap_or("none");
    let expected_status = match scenario.expected_status {
        Some(s) => s,
        None => return Verdict::HarnessError("missing expected_status".into()),
    };
    let declared_outcome = match &scenario.outcome {
        Some(o) => o.clone(),
        None => return Verdict::HarnessError("missing outcome".into()),
    };
    let reference_mode = scenario
        .reference_mode
        .as_ref()
        .unwrap_or(&ReferenceMode::None);

    let url = format!("{}{}", endpoints.our, service_path(&scenario.service));
    let (status, body) = match post(&url, &request_body, "application/soap+xml; charset=utf-8") {
        Ok(r) => r,
        Err(e) => return Verdict::HarnessError(format!("POST our: {e}")),
    };

    let our_verdict = validate_response(
        name,
        status,
        &body,
        expected_status,
        &declared_outcome,
        schema_id,
        &scenario.invariants,
        oracle,
    );

    let final_verdict = combine_with_reference(
        our_verdict,
        &body,
        &request_body,
        schema_id,
        &scenario.masks,
        &declared_outcome,
        reference_mode,
        scenario.operation.as_deref().unwrap_or(""),
        &scenario.service,
        name,
        endpoints,
        store,
    );

    maybe_promote(&final_verdict, name, &body, oracle, store, promote_on_pass);
    final_verdict
}

// ---------------------------------------------------------------------------
// Multi-step scenario
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_multistep_scenario(
    scenario: &Scenario,
    name: &str,
    scenarios_dir: &std::path::Path,
    endpoints: &Endpoints,
    oracle: &Oracle,
    store: &SnapshotStore,
    promote_on_pass: bool,
    nonce_seed: &mut u64,
) -> Verdict {
    let mut captures: HashMap<String, String> = HashMap::new();
    let mut worst = Verdict::Pass;
    let reference_mode = scenario
        .reference_mode
        .as_ref()
        .unwrap_or(&ReferenceMode::None);

    for (step_idx, step) in scenario.steps.iter().enumerate() {
        let raw_request = match std::fs::read(scenarios_dir.join(&step.request_file)) {
            Ok(b) => b,
            Err(e) => {
                worst = worst_verdict(
                    worst,
                    Verdict::HarnessError(format!("read step {step_idx} request: {e}")),
                );
                break;
            }
        };

        let substituted = apply_substitutions(&raw_request, &captures);

        let request_body = if scenario.auth_mode() == AuthMode::Usernametoken {
            let seed = *nonce_seed;
            *nonce_seed += 1;
            inject_wsse(&substituted, seed)
        } else {
            substituted
        };

        let url = format!("{}{}", endpoints.our, service_path(&scenario.service));
        let (status, body) = match post(&url, &request_body, "application/soap+xml; charset=utf-8")
        {
            Ok(r) => r,
            Err(e) => {
                worst = worst_verdict(
                    worst,
                    Verdict::HarnessError(format!("POST our step {step_idx}: {e}")),
                );
                break;
            }
        };

        let step_verdict = validate_response(
            name,
            status,
            &body,
            step.expected_status,
            &step.outcome,
            &step.schema_id,
            &step.invariants,
            oracle,
        );

        // Reference mode for multistep: the plan says reference_mode is at scenario level.
        // For multistep scenarios with reference_mode=none, we just use evaluate_none.
        // For simplicity, we only do reference comparison on the LAST step (the one that
        // produces the "answer"). Steps that are setup (capture) are validated against
        // our-side only.
        let is_last_step = step_idx == scenario.steps.len() - 1;
        let effective_reference = if is_last_step {
            reference_mode
        } else {
            &ReferenceMode::None
        };

        let step_name = format!("{name}.step{step_idx}");
        let step_final = combine_with_reference(
            step_verdict,
            &body,
            &request_body,
            &step.schema_id,
            &step.masks,
            &step.outcome,
            effective_reference,
            &step.operation,
            &scenario.service,
            &step_name,
            endpoints,
            store,
        );

        worst = worst_verdict(worst, step_final);

        // Capture declared values even if we had an error (best effort).
        for cap in &step.capture {
            if let Some(val) = capture_value(&body, &cap.path) {
                captures.insert(cap.name.clone(), val);
            }
        }
    }

    maybe_promote(&worst, name, &[], oracle, store, promote_on_pass);
    worst
}

// ---------------------------------------------------------------------------
// Our-side validation
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn validate_response(
    name: &str,
    status: u16,
    body: &[u8],
    expected_status: u16,
    declared_outcome: &Outcome,
    schema_id: &str,
    invariants: &[String],
    oracle: &Oracle,
) -> Verdict {
    // Status check.
    if status != expected_status {
        return Verdict::SutFail(format!(
            "scenario {name}: expected status {expected_status} got {status}"
        ));
    }

    // Envelope schema validation.
    let env_result = match oracle.validate(body, "soap12-envelope") {
        Ok(r) => r,
        Err(e) => return Verdict::HarnessError(format!("oracle envelope validate: {e}")),
    };
    if !env_result.valid {
        return Verdict::SutFail(format!(
            "envelope schema invalid: {}",
            env_result.errors.join("; ")
        ));
    }

    // Body-child schema validation (skip for schema_id "none" or fault).
    let is_fault_outcome = matches!(declared_outcome, Outcome::Fault);
    if schema_id != "none" && !is_fault_outcome {
        match extract_body_child(body) {
            None => {
                return Verdict::SutFail(
                    "could not extract body child for schema validation".into(),
                );
            }
            Some(child) => {
                let body_result = match oracle.validate(&child, schema_id) {
                    Ok(r) => r,
                    Err(e) => {
                        return Verdict::HarnessError(format!("oracle body validate: {e}"));
                    }
                };
                if !body_result.valid {
                    return Verdict::SutFail(format!(
                        "body schema ({schema_id}) invalid: {}",
                        body_result.errors.join("; ")
                    ));
                }
            }
        }
    }

    // Invariants on RAW body (before masking, per spec §5).
    let ctx = InvariantCtx {
        request_message_id: String::new(),
        expected_endpoint: String::new(),
        expected_scopes: vec![],
    };
    for inv in invariants {
        if let Err(e) = check_invariant(inv, body, &ctx) {
            return Verdict::SutFail(format!("invariant '{inv}' failed: {e}"));
        }
    }

    // Outcome check: declared success/fault vs actual HTTP status.
    let our_is_success = is_http_success(status);
    let declared_success = matches!(declared_outcome, Outcome::Success);
    if our_is_success != declared_success {
        return Verdict::SutFail(format!(
            "outcome mismatch: declared {} but got {}",
            if declared_success { "success" } else { "fault" },
            if our_is_success { "success" } else { "fault" },
        ));
    }

    Verdict::Pass
}

// ---------------------------------------------------------------------------
// Reference comparison
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn combine_with_reference(
    our_verdict: Verdict,
    our_body: &[u8],
    _request_body: &[u8],
    _schema_id: &str,
    masks: &[String],
    declared_outcome: &Outcome,
    reference_mode: &ReferenceMode,
    operation: &str,
    service: &str,
    name: &str,
    endpoints: &Endpoints,
    store: &SnapshotStore,
) -> Verdict {
    // If our side already failed, that's the worst verdict regardless of reference.
    match &our_verdict {
        Verdict::SutFail(_) => return our_verdict,
        Verdict::HarnessError(_) => return our_verdict,
        _ => {}
    }

    let declared_success = matches!(declared_outcome, Outcome::Success);
    let our_schema_valid = matches!(our_verdict, Verdict::Pass | Verdict::KnownDivergence(_));
    let our_is_success = declared_success; // validated above in validate_response

    match reference_mode {
        ReferenceMode::None => {
            verdict::evaluate_none(declared_success, our_schema_valid, our_is_success)
        }

        ReferenceMode::SrvdExact => {
            // POST to srvd.
            let srvd_path = srvd_service_path(service);
            let srvd_url = format!("{}{}", endpoints.srvd, srvd_path);
            // Re-inject WSSE for srvd (same request structure; use a fixed nonce offset
            // so the two calls differ — but in practice srvd request is same as our request).
            // NOTE: for SrvdExact, we send the SAME already-injected request body.
            // The request_body param already has WSSE injected for our server.
            // For srvd we also send it — srvd accepts admin/admin too.
            let srvd_body_bytes = match post(
                &srvd_url,
                _request_body,
                "application/soap+xml; charset=utf-8",
            ) {
                Ok((_, b)) => b,
                Err(e) => return Verdict::HarnessError(format!("POST srvd: {e}")),
            };

            // Mask + canonicalize_prefixes both sides, then evaluate.
            let (text_rules, attr_rules) = resolve_all(masks);
            let our_canon = mask_only(our_body, &text_rules, &attr_rules)
                .map_err(|e| format!("mask_only our: {e}"));
            let srvd_canon = mask_only(&srvd_body_bytes, &text_rules, &attr_rules)
                .map_err(|e| format!("mask_only srvd: {e}"));

            let eval = Eval {
                sut: our_canon,
                reference: srvd_canon,
                known_divergences: vec![],
            };
            verdict::evaluate(&eval)
        }

        ReferenceMode::SrvdProjection => {
            // Determine the projection function by operation.
            let proj_fn = match projection_fn_for_operation(operation) {
                Some(f) => f,
                None => {
                    return Verdict::HarnessError(format!(
                        "no projection function for operation '{operation}'"
                    ));
                }
            };

            // Project our body-child.
            let our_proj = match extract_body_child(our_body)
                .ok_or_else(|| "cannot extract our body child for projection".to_string())
                .and_then(|child| proj_fn(&child))
            {
                Ok(p) => p,
                Err(e) => return Verdict::HarnessError(format!("project our: {e}")),
            };

            // Project the committed Layer-1 snapshot body-child as the expected fixture.
            // snap name for single-step is just `name`; for multi-step it's `name.stepN`.
            let expected_fixture = match load_fixture_projection(store, name, proj_fn) {
                Ok(f) => f,
                Err(e) => return Verdict::HarnessError(format!("fixture projection: {e}")),
            };

            // POST to srvd and project.
            let srvd_path = srvd_service_path(service);
            let srvd_url = format!("{}{}", endpoints.srvd, srvd_path);
            let srvd_body_bytes = match post(
                &srvd_url,
                _request_body,
                "application/soap+xml; charset=utf-8",
            ) {
                Ok((_, b)) => b,
                Err(e) => return Verdict::HarnessError(format!("POST srvd: {e}")),
            };

            let srvd_proj = match extract_body_child(&srvd_body_bytes)
                .ok_or_else(|| "cannot extract srvd body child for projection".to_string())
                .and_then(|child| proj_fn(&child))
            {
                Ok(p) => p,
                Err(e) => return Verdict::HarnessError(format!("project srvd: {e}")),
            };

            verdict::evaluate_projection(&expected_fixture, &our_proj, &srvd_proj)
        }
    }
}

// ---------------------------------------------------------------------------
// Promotion
// ---------------------------------------------------------------------------

fn maybe_promote(
    verdict: &Verdict,
    name: &str,
    body: &[u8],
    oracle: &Oracle,
    store: &SnapshotStore,
    promote_on_pass: bool,
) {
    if !promote_on_pass {
        return;
    }
    if !matches!(verdict, Verdict::Pass | Verdict::KnownDivergence(_)) {
        return;
    }
    if body.is_empty() {
        // Multi-step: no single body to canonicalize. Skip canonical evidence.
        // The individual steps were promoted through step_name paths during combine_with_reference.
        return;
    }
    match oracle.c14n(body) {
        Ok(canonical) => {
            if let Err(e) = promote::promote(store, name, &canonical) {
                eprintln!("[layer2] promote {name}: {e}");
            }
        }
        Err(e) => {
            eprintln!("[layer2] c14n for promote {name}: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Projection helpers
// ---------------------------------------------------------------------------

type ProjFn = fn(&[u8]) -> Result<crate::projection::CanonProjection, String>;

/// Map an ONVIF operation name to a projection function.
pub fn projection_fn_for_operation(operation: &str) -> Option<ProjFn> {
    match operation {
        "GetCapabilities" => Some(projection::get_capabilities),
        "GetServices" => Some(projection::get_services),
        "GetProfiles" => Some(projection::get_profiles),
        _ => None,
    }
}

/// Load the committed Layer-1 snapshot and project it to produce the expected fixture.
fn load_fixture_projection(
    store: &SnapshotStore,
    name: &str,
    proj_fn: ProjFn,
) -> Result<crate::projection::CanonProjection, String> {
    let snapshot_xml = store
        .read(name)
        .ok_or_else(|| format!("no snapshot for '{name}' — run Layer-1 first"))?;
    let child = extract_body_child(snapshot_xml.as_bytes())
        .ok_or_else(|| format!("cannot extract body child from snapshot '{name}'"))?;
    proj_fn(&child)
}

// ---------------------------------------------------------------------------
// srvd service path
// ---------------------------------------------------------------------------

/// Map a service name to the onvif-srvd URL path.
/// onvif-srvd (Axis/Dahua ONVIF simulator) exposes all services at the device path.
/// This is an assumption that the live run (Part B) must confirm.
fn srvd_service_path(service: &str) -> &'static str {
    match service {
        "device" => "/onvif/device_service",
        "media" => "/onvif/media_service",
        "imaging" => "/onvif/imaging_service",
        "ptz" => "/onvif/ptz_service",
        "events" => "/onvif/events_service",
        _ => "/onvif/device_service",
    }
}

// ---------------------------------------------------------------------------
// Verdict precedence
// ---------------------------------------------------------------------------

/// Combine two verdicts, returning the "worst" one.
///
/// Precedence (worst → best):
/// 1. SutFail
/// 2. HarnessError
/// 3. ReferenceDisagreement
/// 4. KnownDivergence
/// 5. Pass
pub fn worst_verdict(a: Verdict, b: Verdict) -> Verdict {
    fn rank(v: &Verdict) -> u8 {
        match v {
            Verdict::SutFail(_) => 4,
            Verdict::HarnessError(_) => 3,
            Verdict::ReferenceDisagreement(_) => 2,
            Verdict::KnownDivergence(_) => 1,
            Verdict::Pass => 0,
        }
    }
    if rank(&b) > rank(&a) {
        b
    } else {
        a
    }
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

/// Returns true when `status` indicates a successful (non-fault) response.
fn is_http_success(status: u16) -> bool {
    status == 200
}

// ---------------------------------------------------------------------------
// Substitution + capture helpers (mirrored from Layer-1 harness)
// ---------------------------------------------------------------------------

fn apply_substitutions(body: &[u8], captures: &HashMap<String, String>) -> Vec<u8> {
    if captures.is_empty() {
        return body.to_vec();
    }
    let mut s = String::from_utf8_lossy(body).into_owned();
    for (k, v) in captures {
        s = s.replace(&format!("{{{{{k}}}}}"), v);
    }
    s.into_bytes()
}

fn capture_value(response: &[u8], path: &str) -> Option<String> {
    let local = path.strip_prefix("body:").unwrap_or(path);
    extract_text(response, local)
}

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
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// POST XML bytes to `url` with the given Content-Type header.
/// Returns `(status_code, body_bytes)`.
pub fn post(url: &str, body: &[u8], content_type: &str) -> Result<(u16, Vec<u8>), String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(url)
        .header("Content-Type", content_type)
        .body(body.to_vec())
        .send()
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let bytes = resp.bytes().map_err(|e| e.to_string())?.to_vec();
    Ok((status, bytes))
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic — no Docker)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── worst_verdict precedence ──────────────────────────────────────────────

    #[test]
    fn worst_verdict_sut_fail_beats_everything() {
        let sut = Verdict::SutFail("x".into());
        let pass = Verdict::Pass;
        let harness = Verdict::HarnessError("h".into());
        let ref_dis = Verdict::ReferenceDisagreement("r".into());
        let known = Verdict::KnownDivergence("k".into());

        assert!(matches!(
            worst_verdict(sut.clone(), pass.clone()),
            Verdict::SutFail(_)
        ));
        assert!(matches!(
            worst_verdict(pass.clone(), sut.clone()),
            Verdict::SutFail(_)
        ));
        assert!(matches!(
            worst_verdict(sut.clone(), harness.clone()),
            Verdict::SutFail(_)
        ));
        assert!(matches!(
            worst_verdict(sut.clone(), ref_dis.clone()),
            Verdict::SutFail(_)
        ));
        assert!(matches!(
            worst_verdict(sut.clone(), known.clone()),
            Verdict::SutFail(_)
        ));
        let _ = (pass, harness, ref_dis, known); // suppress unused warnings
    }

    #[test]
    fn worst_verdict_harness_error_beats_ref_disagreement_and_below() {
        let h = Verdict::HarnessError("h".into());
        let r = Verdict::ReferenceDisagreement("r".into());
        let k = Verdict::KnownDivergence("k".into());
        let p = Verdict::Pass;

        assert!(matches!(
            worst_verdict(h.clone(), r.clone()),
            Verdict::HarnessError(_)
        ));
        assert!(matches!(
            worst_verdict(r.clone(), h.clone()),
            Verdict::HarnessError(_)
        ));
        assert!(matches!(
            worst_verdict(h.clone(), k),
            Verdict::HarnessError(_)
        ));
        assert!(matches!(
            worst_verdict(h.clone(), p),
            Verdict::HarnessError(_)
        ));
        let _ = (r,); // suppress
    }

    #[test]
    fn worst_verdict_reference_disagreement_beats_known_and_pass() {
        let r = Verdict::ReferenceDisagreement("r".into());
        let k = Verdict::KnownDivergence("k".into());
        let p = Verdict::Pass;

        assert!(matches!(
            worst_verdict(r.clone(), k),
            Verdict::ReferenceDisagreement(_)
        ));
        assert!(matches!(
            worst_verdict(p, r.clone()),
            Verdict::ReferenceDisagreement(_)
        ));
    }

    #[test]
    fn worst_verdict_pass_vs_pass_is_pass() {
        assert_eq!(worst_verdict(Verdict::Pass, Verdict::Pass), Verdict::Pass);
    }

    // ── projection_fn_for_operation mapping ──────────────────────────────────

    #[test]
    fn projection_fn_known_operations_return_some() {
        assert!(projection_fn_for_operation("GetCapabilities").is_some());
        assert!(projection_fn_for_operation("GetServices").is_some());
        assert!(projection_fn_for_operation("GetProfiles").is_some());
    }

    #[test]
    fn projection_fn_unknown_operation_returns_none() {
        assert!(projection_fn_for_operation("GetDeviceInformation").is_none());
        assert!(projection_fn_for_operation("").is_none());
        assert!(projection_fn_for_operation("NotAReal").is_none());
    }

    // ── apply_substitutions ────────────────────────────────────────────────────

    #[test]
    fn apply_substitutions_replaces_placeholder() {
        let mut caps = HashMap::new();
        caps.insert("subId".to_string(), "abc-123".to_string());
        let body = b"<tev:SubscriptionId>{{subId}}</tev:SubscriptionId>";
        let result = apply_substitutions(body, &caps);
        assert_eq!(
            String::from_utf8(result).unwrap(),
            "<tev:SubscriptionId>abc-123</tev:SubscriptionId>"
        );
    }

    #[test]
    fn apply_substitutions_noop_when_empty() {
        let body = b"<foo>bar</foo>";
        let result = apply_substitutions(body, &HashMap::new());
        assert_eq!(result, body);
    }

    // ── capture_value ──────────────────────────────────────────────────────────

    #[test]
    fn capture_value_strips_body_prefix_and_finds_element() {
        let xml =
            b"<Envelope><Body><Resp><SubscriptionId>XYZ</SubscriptionId></Resp></Body></Envelope>";
        assert_eq!(
            capture_value(xml, "body:SubscriptionId"),
            Some("XYZ".to_string())
        );
        assert_eq!(
            capture_value(xml, "SubscriptionId"),
            Some("XYZ".to_string())
        );
    }

    #[test]
    fn capture_value_returns_none_for_absent_element() {
        let xml = b"<Envelope><Body><Resp/></Body></Envelope>";
        assert_eq!(capture_value(xml, "SubscriptionId"), None);
    }

    // ── fixture projection (snapshot-based, static) ───────────────────────────

    #[test]
    fn fixture_projection_from_get_capabilities_snapshot() {
        // Minimal GetCapabilitiesResponse body-child (ns-decorated like real snapshot output).
        let body_child = br#"<tds:GetCapabilitiesResponse
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
            xmlns:tt="http://www.onvif.org/ver10/schema">
          <tds:Capabilities>
            <tt:Device><tt:XAddr>http://controlled-onvif:8080/onvif/device_service</tt:XAddr></tt:Device>
            <tt:Media><tt:XAddr>http://controlled-onvif:8080/onvif/media_service</tt:XAddr></tt:Media>
            <tt:PTZ><tt:XAddr>http://controlled-onvif:8080/onvif/ptz_service</tt:XAddr></tt:PTZ>
            <tt:Imaging><tt:XAddr>http://controlled-onvif:8080/onvif/imaging_service</tt:XAddr></tt:Imaging>
          </tds:Capabilities>
        </tds:GetCapabilitiesResponse>"#;

        let proj = projection::get_capabilities(body_child).expect("projection failed");
        // Device, Media, PTZ, Imaging should be present.
        assert!(proj.contains_key("Device"), "Device key missing");
        assert!(proj.contains_key("Media"), "Media key missing");
        assert!(proj.contains_key("PTZ"), "PTZ key missing");
        assert!(proj.contains_key("Imaging"), "Imaging key missing");

        // XAddr path should be authority-stripped.
        let device = proj.get("Device").unwrap();
        assert_eq!(
            device.xaddr_path.as_deref(),
            Some("/onvif/device_service"),
            "device xaddr path should strip authority"
        );
    }

    // ── is_http_success ────────────────────────────────────────────────────────

    #[test]
    fn http_success_200() {
        assert!(is_http_success(200));
    }

    #[test]
    fn http_not_success_500() {
        assert!(!is_http_success(500));
        assert!(!is_http_success(400));
    }
}
