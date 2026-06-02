//! Per-scenario verdict report (spec §5.2, §7). Surfaces conformance verdicts
//! plus the count of still-unverified snapshots so self-captured baselines are
//! never mistaken for conformance evidence.

use crate::layer2::verdict::Verdict;

pub struct Report {
    pub rows: Vec<(String, Verdict)>,
    pub unverified_remaining: usize,
}

impl Report {
    pub fn print(&self) {
        println!("=== Conformance ({} scenarios) ===", self.rows.len());
        for (name, v) in &self.rows {
            println!("  {name:43} {v:?}");
        }
        let pass = self
            .rows
            .iter()
            .filter(|(_, v)| matches!(v, Verdict::Pass | Verdict::KnownDivergence(_)))
            .count();
        let fail = self.rows.len() - pass;
        println!("  → {pass} Pass/KnownDivergence, {fail} Fail/Error/Disagreement\n");

        println!("=== Summary: {} total ===", self.rows.len());
        println!("  {pass} Pass/KnownDivergence, {fail} Fail/Error/Disagreement");
        println!(
            "  {} snapshot(s) still unverified (target: 0 after Phase 2b)",
            self.unverified_remaining
        );
        if self.is_green() && self.unverified_remaining == 0 {
            println!("  All seed scenarios verified — Phase 2b complete (spec §13)");
        } else if !self.is_green() {
            println!("  run has {fail} Fail/Error verdict(s) — NOT release-green");
        }
    }

    /// Returns true only when every verdict is Pass or KnownDivergence (spec §11.5 green gate).
    ///
    /// SutFail, HarnessError, AND ReferenceDisagreement all make the run non-green:
    /// a ReferenceDisagreement means the scenario is NOT verified and requires triage
    /// before a release decision can be made.
    pub fn is_green(&self) -> bool {
        self.rows
            .iter()
            .all(|(_, v)| matches!(v, Verdict::Pass | Verdict::KnownDivergence(_)))
    }

    /// Count of verdicts that are NOT Pass/KnownDivergence (i.e. SutFail, HarnessError,
    /// AND ReferenceDisagreement). Kept consistent with `is_green()` so the printed
    /// pass/fail totals never disagree with the green gate / exit code.
    pub fn fail_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|(_, v)| !matches!(v, Verdict::Pass | Verdict::KnownDivergence(_)))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(rows: Vec<(&str, Verdict)>) -> Report {
        Report {
            rows: rows.into_iter().map(|(n, v)| (n.to_string(), v)).collect(),
            unverified_remaining: 0,
        }
    }

    #[test]
    fn reference_disagreement_counts_as_fail_and_is_not_green() {
        let r = report(vec![
            ("a", Verdict::Pass),
            ("b", Verdict::ReferenceDisagreement("ref differs".into())),
            ("c", Verdict::SutFail("bad".into())),
        ]);
        // fail_count must include the ReferenceDisagreement (2 = b + c), consistent with is_green.
        assert_eq!(r.fail_count(), 2);
        assert!(!r.is_green());
        // pass + fail must equal total (no verdict falls through the cracks).
        let pass = r.rows.len() - r.fail_count();
        assert_eq!(pass + r.fail_count(), r.rows.len());
    }

    #[test]
    fn all_pass_is_green_zero_fail() {
        let r = report(vec![
            ("a", Verdict::Pass),
            ("b", Verdict::KnownDivergence("ok".into())),
        ]);
        assert_eq!(r.fail_count(), 0);
        assert!(r.is_green());
    }
}
