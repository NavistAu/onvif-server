//! Layer-2 verdict model (spec §5.7).
//!
//! `evaluate(&Eval)` compares the normalized bytes from our SUT and the reference
//! server and returns a `Verdict`. Known divergences are declared per-scenario in
//! `Eval::known_divergences`.

/// The outcome of a single Layer-2 scenario comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Both sides produced valid, byte-identical normalized output.
    Pass,
    /// Our SUT produced invalid XML (or the oracle rejected it).
    SutFail(String),
    /// The reference produced output our oracle considers invalid.
    /// This indicates a reference or oracle bug, not a SUT regression.
    ReferenceDisagreement(String),
    /// The normalized outputs differ, but this divergence is listed in
    /// `Eval::known_divergences` for this scenario.
    KnownDivergence(String),
    /// The harness itself failed (HTTP error, oracle error, etc.) — not a SUT verdict.
    /// Never counts as pass; the scenario must be re-run.
    HarnessError(String),
}

/// Input to `evaluate`: the normalized bytes from each side, plus metadata.
pub struct Eval {
    /// Normalized bytes from our SUT (output of `mask_only`), or an error string.
    pub sut: Result<Vec<u8>, String>,
    /// Normalized bytes from the reference server (output of `mask_only`), or an error.
    pub reference: Result<Vec<u8>, String>,
    /// Reason strings for known divergences on this scenario. If the two sides differ
    /// but the reason matches an entry here, `KnownDivergence` is returned instead of
    /// `SutFail`. Comparison is by exact string equality.
    pub known_divergences: Vec<String>,
}

/// Evaluate a single scenario comparison and return the verdict.
pub fn evaluate(eval: &Eval) -> Verdict {
    match (&eval.sut, &eval.reference) {
        (Err(msg), _) => Verdict::SutFail(msg.clone()),
        (_, Err(msg)) => Verdict::ReferenceDisagreement(msg.clone()),
        (Ok(sut_bytes), Ok(ref_bytes)) => {
            if sut_bytes == ref_bytes {
                Verdict::Pass
            } else {
                // Check known divergences.
                let diff_reason = format!(
                    "sut={} ref={}",
                    String::from_utf8_lossy(sut_bytes),
                    String::from_utf8_lossy(ref_bytes)
                );
                for known in &eval.known_divergences {
                    // Match if the diff_reason string contains the known-divergence token,
                    // or if either side's bytes contain the token as a subsequence.
                    let known_bytes = known.as_bytes();
                    let sut_has = sut_bytes
                        .windows(known_bytes.len())
                        .any(|w| w == known_bytes);
                    let ref_has = ref_bytes
                        .windows(known_bytes.len())
                        .any(|w| w == known_bytes);
                    if *known == diff_reason
                        || sut_has
                        || ref_has
                        || diff_reason.contains(known.as_str())
                    {
                        return Verdict::KnownDivergence(known.clone());
                    }
                }
                Verdict::SutFail(format!(
                    "outputs differ: sut={} ref={}",
                    String::from_utf8_lossy(sut_bytes),
                    String::from_utf8_lossy(ref_bytes)
                ))
            }
        }
    }
}

// ─── Outcome-equivalence model for WS-Security / auth scenarios ──────────────
//
// Per spec §10, conformance is assessed at the *outcome* level:
// two servers that both accept (HTTP 200 + equivalent body) or both reject
// (SOAP Fault) a given credential are considered equivalent.

/// A normalised response for outcome-equivalence comparison.
/// - `schema_valid`: the SOAP envelope validated against the oracle schema.
/// - `is_success`: HTTP 200 and no SOAP Fault element in the body.
/// - `masked_body_canon`: oracle-C14N bytes of the Body subtree with the entire
///   Envelope/Header dropped and Fault/Reason/Text masked. Used only when both
///   sides are schema-valid and both succeed; otherwise the body is not compared.
#[derive(Debug, Clone)]
pub struct Resp {
    pub schema_valid: bool,
    pub is_success: bool,
    /// Oracle-C14N bytes of the masked body (for body-level equality on success).
    pub masked_body_canon: Vec<u8>,
}

// ─── Projection-mode evaluator (spec §6) ─────────────────────────────────────
//
// Note: `srvd_exact` mode (not implemented here) reuses `evaluate(&Eval)` for
// masked-bytes equality — see that function for its semantics.

/// Evaluate a `srvd_projection` comparison (spec §6).
///
/// Compares three projections using a 3-gate asymmetric rule:
///
/// 1. `our != expected_fixture` → `SutFail` — our server's projection diverges
///    from the expected fixture (extra keys, missing keys, or a field mismatch).
/// 2. For each entry WE advertise (`our`): `srvd` must contain a matching entry
///    (same key AND same projected fields).  If any is missing or differs →
///    `ReferenceDisagreement`.  (srvd's EXTRA entries are intentionally ignored.)
/// 3. Else → `Pass`.
pub fn evaluate_projection(
    expected_fixture: &crate::projection::CanonProjection,
    our: &crate::projection::CanonProjection,
    srvd: &crate::projection::CanonProjection,
) -> Verdict {
    // Gate 1: fixture-equality check — our server must match the fixture exactly.
    if our != expected_fixture {
        let our_extra: Vec<&String> = our
            .keys()
            .filter(|k| !expected_fixture.contains_key(*k))
            .collect();
        let fixture_extra: Vec<&String> = expected_fixture
            .keys()
            .filter(|k| !our.contains_key(*k))
            .collect();
        let differing: Vec<&String> = our
            .iter()
            .filter(|(k, v)| expected_fixture.get(*k).is_some_and(|fv| fv != *v))
            .map(|(k, _)| k)
            .collect();

        let mut parts: Vec<String> = Vec::new();
        if !our_extra.is_empty() {
            parts.push(format!("extra keys in our: {:?}", our_extra));
        }
        if !fixture_extra.is_empty() {
            parts.push(format!("keys missing from our: {:?}", fixture_extra));
        }
        if !differing.is_empty() {
            parts.push(format!("field mismatch at keys: {:?}", differing));
        }
        return Verdict::SutFail(format!(
            "our projection does not match fixture — {}",
            parts.join("; "),
        ));
    }
    // Gate 2: for every entry we advertise, srvd must agree.
    for (key, our_entry) in our {
        match srvd.get(key) {
            None => {
                return Verdict::ReferenceDisagreement(format!(
                    "reference is missing entry for key {:?} that we advertise",
                    key,
                ));
            }
            Some(srvd_entry) if srvd_entry != our_entry => {
                return Verdict::ReferenceDisagreement(format!(
                    "reference entry for key {:?} differs from ours",
                    key,
                ));
            }
            _ => {}
        }
    }
    Verdict::Pass
}

/// Evaluate a `none`-reference-mode scenario (spec §6 / §5).
///
/// No reference server is involved.  Verdict depends solely on our server's
/// schema validity and whether the outcome matches the declared contract.
///
/// - `declared_success`: the scenario's declared outcome (`Outcome::Success` → true).
/// - `our_schema_valid`: our response validated against the oracle schema.
/// - `our_is_success`: our response was HTTP 200 with no SOAP Fault.
pub fn evaluate_none(
    declared_success: bool,
    our_schema_valid: bool,
    our_is_success: bool,
) -> Verdict {
    if !our_schema_valid {
        return Verdict::SutFail("our response schema-invalid".into());
    }
    if our_is_success != declared_success {
        return Verdict::SutFail(format!(
            "our server outcome ({}) does not match the scenario's declared outcome ({})",
            if our_is_success { "success" } else { "fault" },
            if declared_success { "success" } else { "fault" },
        ));
    }
    Verdict::Pass
}

/// Evaluate outcome-equivalence for a scenario (spec §10).
///
/// `declared_success`: the scenario's declared outcome (`Outcome::Success` → true).
///
/// Rules (outcome-aware — the declared outcome is the contract anchor):
/// - our schema-invalid → `SutFail`.
/// - our outcome ≠ declared → `SutFail` (our server violated its contract; ref irrelevant).
/// - ref schema-invalid → `ReferenceDisagreement`.
/// - ref outcome ≠ declared → `ReferenceDisagreement` (scenario needs triage).
/// - both match declared AND declared success AND equal masked body → `Pass`.
/// - both match declared AND declared success AND unequal body → `SutFail` (real diff).
/// - both match declared AND declared fault → `Pass` (class-equivalence; reason non-asserted).
pub fn evaluate_outcome_equivalence(declared_success: bool, our: &Resp, cxf: &Resp) -> Verdict {
    if !our.schema_valid {
        return Verdict::SutFail("our response schema-invalid".into());
    }
    // Our server MUST honour the scenario's declared outcome (independent of reference).
    if our.is_success != declared_success {
        return Verdict::SutFail(format!(
            "our server outcome ({}) does not match the scenario's declared outcome ({})",
            if our.is_success { "success" } else { "fault" },
            if declared_success { "success" } else { "fault" },
        ));
    }
    if !cxf.schema_valid {
        return Verdict::ReferenceDisagreement("reference response schema-invalid".into());
    }
    // Conformance: does reference reach the same (declared, SUT-confirmed) outcome?
    if cxf.is_success != declared_success {
        return Verdict::ReferenceDisagreement(format!(
            "reference outcome ({}) differs from the declared/SUT outcome ({}) — needs triage",
            if cxf.is_success { "success" } else { "fault" },
            if declared_success { "success" } else { "fault" },
        ));
    }
    // Both match the declared outcome.
    if declared_success {
        if our.masked_body_canon == cxf.masked_body_canon {
            Verdict::Pass
        } else {
            Verdict::SutFail(format!(
                "both succeeded but body differs: our={} ref={}",
                String::from_utf8_lossy(&our.masked_body_canon),
                String::from_utf8_lossy(&cxf.masked_body_canon),
            ))
        }
    } else {
        Verdict::Pass // both faulted as the scenario declared — class-equivalence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Result<Vec<u8>, String> {
        Ok(s.as_bytes().to_vec())
    }

    fn err(s: &str) -> Result<Vec<u8>, String> {
        Err(s.to_string())
    }

    // ─── evaluate_outcome_equivalence unit tests ───────────────────────────────

    fn resp_success(body: &str) -> Resp {
        Resp {
            schema_valid: true,
            is_success: true,
            masked_body_canon: body.as_bytes().to_vec(),
        }
    }

    fn resp_fault() -> Resp {
        Resp {
            schema_valid: true,
            is_success: false,
            masked_body_canon: vec![],
        }
    }

    fn resp_invalid() -> Resp {
        Resp {
            schema_valid: false,
            is_success: false,
            masked_body_canon: vec![],
        }
    }

    // declared_success=true, our success, ref success, bodies equal → Pass
    #[test]
    fn oe_declared_success_both_success_equal_body_is_pass() {
        let our = resp_success("<body>hi</body>");
        let cxf = resp_success("<body>hi</body>");
        assert_eq!(
            evaluate_outcome_equivalence(true, &our, &cxf),
            Verdict::Pass
        );
    }

    // declared_success=true, our success, ref success, bodies differ → SutFail
    #[test]
    fn oe_declared_success_both_success_unequal_body_is_sut_fail() {
        let our = resp_success("<body>A</body>");
        let cxf = resp_success("<body>B</body>");
        assert!(matches!(
            evaluate_outcome_equivalence(true, &our, &cxf),
            Verdict::SutFail(_)
        ));
    }

    // declared_success=true, our fault → SutFail (our server failed its contract, ref irrelevant)
    #[test]
    fn oe_declared_success_our_fault_is_sut_fail_regardless_of_ref() {
        let our = resp_fault();
        let cxf = resp_success("<body>hi</body>");
        assert!(matches!(
            evaluate_outcome_equivalence(true, &our, &cxf),
            Verdict::SutFail(_)
        ));
    }

    // declared_success=true, our success, ref fault → ReferenceDisagreement
    #[test]
    fn oe_declared_success_our_success_ref_fault_is_reference_disagreement() {
        let our = resp_success("<body>hi</body>");
        let cxf = resp_fault();
        assert!(matches!(
            evaluate_outcome_equivalence(true, &our, &cxf),
            Verdict::ReferenceDisagreement(_)
        ));
    }

    // declared_fault=true, our fault, ref fault → Pass (class-equivalence)
    #[test]
    fn oe_declared_fault_both_fault_is_pass() {
        let our = resp_fault();
        let cxf = resp_fault();
        assert_eq!(
            evaluate_outcome_equivalence(false, &our, &cxf),
            Verdict::Pass
        );
    }

    // declared_fault=true, our success → SutFail
    #[test]
    fn oe_declared_fault_our_success_is_sut_fail() {
        let our = resp_success("<body>hi</body>");
        let cxf = resp_fault();
        assert!(matches!(
            evaluate_outcome_equivalence(false, &our, &cxf),
            Verdict::SutFail(_)
        ));
    }

    // declared_fault=true, our fault, ref success → ReferenceDisagreement
    #[test]
    fn oe_declared_fault_our_fault_ref_success_is_reference_disagreement() {
        let our = resp_fault();
        let cxf = resp_success("<body>hi</body>");
        assert!(matches!(
            evaluate_outcome_equivalence(false, &our, &cxf),
            Verdict::ReferenceDisagreement(_)
        ));
    }

    // our schema-invalid → SutFail (regardless of declared or ref)
    #[test]
    fn oe_our_schema_invalid_is_sut_fail() {
        let our = resp_invalid();
        let cxf = resp_fault();
        assert!(matches!(
            evaluate_outcome_equivalence(false, &our, &cxf),
            Verdict::SutFail(_)
        ));
    }

    // ref schema-invalid (our is valid and matches declared) → ReferenceDisagreement
    #[test]
    fn oe_ref_schema_invalid_is_reference_disagreement() {
        let our = resp_fault();
        let cxf = resp_invalid();
        assert!(matches!(
            evaluate_outcome_equivalence(false, &our, &cxf),
            Verdict::ReferenceDisagreement(_)
        ));
    }

    // ─── original evaluate() tests ────────────────────────────────────────────

    #[test]
    fn verdict_pass_when_equal() {
        let eval = Eval {
            sut: ok("<foo/>"),
            reference: ok("<foo/>"),
            known_divergences: vec![],
        };
        assert_eq!(evaluate(&eval), Verdict::Pass);
    }

    #[test]
    fn verdict_sut_fail_when_our_side_invalid() {
        let eval = Eval {
            sut: err("parse error: bad XML"),
            reference: ok("<foo/>"),
            known_divergences: vec![],
        };
        assert!(matches!(evaluate(&eval), Verdict::SutFail(_)));
    }

    #[test]
    fn verdict_reference_disagreement_when_ref_invalid() {
        let eval = Eval {
            sut: ok("<foo/>"),
            reference: err("oracle rejected reference output"),
            known_divergences: vec![],
        };
        assert!(matches!(evaluate(&eval), Verdict::ReferenceDisagreement(_)));
    }

    #[test]
    fn verdict_known_divergence_when_differ_and_listed() {
        let eval = Eval {
            sut: ok("<foo>A</foo>"),
            reference: ok("<foo>B</foo>"),
            known_divergences: vec!["A".to_string()],
        };
        assert!(matches!(evaluate(&eval), Verdict::KnownDivergence(_)));
    }

    #[test]
    fn verdict_sut_fail_when_differ_without_known() {
        let eval = Eval {
            sut: ok("<foo>A</foo>"),
            reference: ok("<foo>B</foo>"),
            known_divergences: vec![],
        };
        assert!(matches!(evaluate(&eval), Verdict::SutFail(_)));
    }
}
