//! Expected-failures baseline and drift-gate logic (Task 7).
//!
//! Loads `crossref/expected-failures.toml` and compares the set of actually-failing
//! scenario names from a run against the recorded expected failures.  The three
//! possible outcomes:
//!
//! - `DriftResult::Clean`      — actual == expected (only known findings red).
//! - `DriftResult::Regression` — new failures not in expected set.
//! - `DriftResult::Stale`      — expected entries that now pass (finding fixed).
//!
//! Both `Regression` and `Stale` are non-zero-exit conditions for CI.

use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

// ---------------------------------------------------------------------------
// TOML types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ExpectedFailure {
    pub scenario: String,
    pub finding: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedFailuresFile {
    /// Defaulted so a comments-only (empty) baseline — the release-green state —
    /// deserializes to an empty list instead of erroring on a missing field.
    #[serde(default)]
    expected_failure: Vec<ExpectedFailure>,
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

/// Load the expected-failures baseline from `crossref/expected-failures.toml`.
pub fn load(crossref_dir: &Path) -> Result<Vec<ExpectedFailure>, String> {
    let path = crossref_dir.join("expected-failures.toml");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed: ExpectedFailuresFile =
        toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok(parsed.expected_failure)
}

// ---------------------------------------------------------------------------
// Drift computation (pure — no I/O)
// ---------------------------------------------------------------------------

/// The outcome of comparing actual failures against the expected-failures baseline.
#[derive(Debug, PartialEq, Eq)]
pub enum DriftResult {
    /// Actual failures == expected failures.  Suite healthy.
    Clean,
    /// New failures discovered that are not in the expected set.
    Regression { new_failures: Vec<String> },
    /// Expected failures that now pass — the baseline is stale.
    Stale { now_passing: Vec<String> },
    /// Both regressions AND stale entries detected simultaneously.
    Both {
        new_failures: Vec<String>,
        now_passing: Vec<String>,
    },
}

/// Compute the drift between `actual_failing` scenario names and the
/// `expected` baseline names.
///
/// `actual_failing` — scenario names whose verdict is **not** Pass/KnownDivergence.
/// `expected`       — scenario names from `expected-failures.toml`.
///
/// Note: multi-step scenarios appear in the run as a single name (e.g.
/// `events_pull_messages`) but the snapshot keys may carry `.stepN`.  We strip
/// any `.stepN` suffix when comparing so the toml entry `events_pull_messages`
/// matches the run row name `events_pull_messages`.
pub fn compute_drift(actual_failing: &HashSet<String>, expected: &HashSet<String>) -> DriftResult {
    // Strip .stepN suffix for comparison.
    let actual_normalized: HashSet<String> = actual_failing
        .iter()
        .map(|s| strip_step_suffix(s))
        .collect();

    let mut new_failures: Vec<String> = actual_normalized
        .iter()
        .filter(|n| !expected.contains(*n))
        .cloned()
        .collect();
    new_failures.sort();

    let mut now_passing: Vec<String> = expected
        .iter()
        .filter(|n| !actual_normalized.contains(*n))
        .cloned()
        .collect();
    now_passing.sort();

    match (new_failures.is_empty(), now_passing.is_empty()) {
        (true, true) => DriftResult::Clean,
        (false, true) => DriftResult::Regression { new_failures },
        (true, false) => DriftResult::Stale { now_passing },
        (false, false) => DriftResult::Both {
            new_failures,
            now_passing,
        },
    }
}

fn strip_step_suffix(name: &str) -> String {
    // Strip a trailing `.stepN` or `.step<N>` from a scenario name.
    if let Some(dot_pos) = name.rfind('.') {
        let suffix = &name[dot_pos + 1..];
        if suffix.starts_with("step") && suffix[4..].chars().all(|c| c.is_ascii_digit()) {
            return name[..dot_pos].to_string();
        }
    }
    name.to_string()
}

// ---------------------------------------------------------------------------
// Print helpers (for the binary)
// ---------------------------------------------------------------------------

impl DriftResult {
    /// Returns `true` when the run has no drift (only known findings red).
    pub fn is_clean(&self) -> bool {
        matches!(self, DriftResult::Clean)
    }

    /// Print a human-readable drift summary.
    pub fn print_summary(&self) {
        match self {
            DriftResult::Clean => {
                println!("[drift] OK — actual failures match expected-failures baseline (only known findings red)");
            }
            DriftResult::Regression { new_failures } => {
                println!(
                    "[drift] REGRESSION/NEW FINDING: {}",
                    new_failures.join(", ")
                );
                println!("[drift] These scenarios fail but are NOT in expected-failures.toml.");
                println!(
                    "[drift] If this is a new product bug, document it in PHASE2B-FINDINGS.md"
                );
                println!("[drift] and add it to expected-failures.toml.");
            }
            DriftResult::Stale { now_passing } => {
                println!(
                    "[drift] STALE BASELINE: {} now pass — remove from expected-failures.toml (finding fixed)",
                    now_passing.join(", ")
                );
            }
            DriftResult::Both {
                new_failures,
                now_passing,
            } => {
                println!(
                    "[drift] REGRESSION/NEW FINDING: {}",
                    new_failures.join(", ")
                );
                println!(
                    "[drift] STALE BASELINE: {} now pass — remove from expected-failures.toml (finding fixed)",
                    now_passing.join(", ")
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests (pure — no I/O)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_only_baseline_parses_to_empty() {
        // The release-green state: a baseline with no [[expected_failure]] blocks.
        let f: ExpectedFailuresFile = toml::from_str("# all findings resolved\n").unwrap();
        assert!(f.expected_failure.is_empty());
    }

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn drift_clean_when_actual_equals_expected() {
        let actual = set(&["ptz_get_status", "device_get_capabilities"]);
        let expected = set(&["ptz_get_status", "device_get_capabilities"]);
        assert_eq!(compute_drift(&actual, &expected), DriftResult::Clean);
    }

    #[test]
    fn drift_regression_when_new_failure_not_in_expected() {
        let actual = set(&["ptz_get_status", "some_new_failure"]);
        let expected = set(&["ptz_get_status"]);
        assert_eq!(
            compute_drift(&actual, &expected),
            DriftResult::Regression {
                new_failures: vec!["some_new_failure".to_string()],
            }
        );
    }

    #[test]
    fn drift_stale_when_expected_now_passes() {
        let actual = set(&["ptz_get_status"]);
        let expected = set(&["ptz_get_status", "device_get_capabilities"]);
        assert_eq!(
            compute_drift(&actual, &expected),
            DriftResult::Stale {
                now_passing: vec!["device_get_capabilities".to_string()],
            }
        );
    }

    #[test]
    fn drift_both_when_regression_and_stale() {
        let actual = set(&["new_failure"]);
        let expected = set(&["old_finding"]);
        assert_eq!(
            compute_drift(&actual, &expected),
            DriftResult::Both {
                new_failures: vec!["new_failure".to_string()],
                now_passing: vec!["old_finding".to_string()],
            }
        );
    }

    #[test]
    fn drift_clean_when_both_empty() {
        assert_eq!(
            compute_drift(&HashSet::new(), &HashSet::new()),
            DriftResult::Clean
        );
    }

    #[test]
    fn drift_step_suffix_stripped_for_multistep() {
        // events_pull_messages appears as one row; the toml entry is "events_pull_messages"
        let actual = set(&["events_pull_messages"]);
        let expected = set(&["events_pull_messages"]);
        assert_eq!(compute_drift(&actual, &expected), DriftResult::Clean);
    }

    #[test]
    fn drift_regression_sorted() {
        let actual = set(&["z_fail", "a_fail", "m_fail"]);
        let expected = set(&[]);
        match compute_drift(&actual, &expected) {
            DriftResult::Regression { new_failures } => {
                assert_eq!(new_failures, vec!["a_fail", "m_fail", "z_fail"]);
            }
            other => panic!("expected Regression, got {other:?}"),
        }
    }

    #[test]
    fn drift_stale_sorted() {
        let actual = set(&[]);
        let expected = set(&["z_old", "a_old"]);
        match compute_drift(&actual, &expected) {
            DriftResult::Stale { now_passing } => {
                assert_eq!(now_passing, vec!["a_old", "z_old"]);
            }
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn strip_step_suffix_strips_step0() {
        assert_eq!(
            strip_step_suffix("events_pull_messages.step0"),
            "events_pull_messages"
        );
        assert_eq!(
            strip_step_suffix("events_pull_messages.step1"),
            "events_pull_messages"
        );
    }

    #[test]
    fn strip_step_suffix_leaves_normal_names() {
        assert_eq!(strip_step_suffix("ptz_get_status"), "ptz_get_status");
        assert_eq!(strip_step_suffix("device.something"), "device.something");
    }
}
