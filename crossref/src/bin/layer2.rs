//! Layer-2 conformance orchestrator — validates/invariants/srvd-compare/promotes.
//!
//! # Usage
//! ```sh
//! cargo run -p onvif-crossref --bin layer2 -- [OPTIONS]
//! ```
//!
//! # Options
//! `--promote`              Promote passing scenarios to `verified` status.
//! `--keep-up`              Leave containers running after the run (default: tear down).
//! `--scenarios <list>`     Comma-separated list of scenario names to run (default: all).
//! `--check-drift`          After the run, compare failing scenarios against the expected-failures
//!                          baseline in `crossref/expected-failures.toml`.  Exits non-zero if
//!                          there are regressions or stale entries.
//!
//! # Staging
//! Before `Topology::up`, the soap-server source tree is staged into
//! `crossref/.build/soap-server/` so the controlled-server Docker image can build it.

use onvif_crossref::drift;
use onvif_crossref::layer2::{compose::Topology, run, Endpoints};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flags = parse_flags(&args[1..]);

    // Derive repo root from CARGO_MANIFEST_DIR (the crossref crate root), then go up one.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
        // When invoked as a compiled binary, CARGO_MANIFEST_DIR may not be set.
        // Fall back to the binary's directory minus two levels (target/debug/…).
        let exe = std::env::current_exe().expect("cannot determine executable path");
        // Typical path: <repo>/target/debug/layer2 → go up 3 levels.
        exe.parent() // debug/
            .and_then(|p| p.parent()) // target/
            .and_then(|p| p.parent()) // repo root
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".to_string())
    });

    // CARGO_MANIFEST_DIR points to the crossref crate; repo root is one level up.
    let crossref_dir = std::path::PathBuf::from(&manifest_dir);
    let repo_root = crossref_dir
        .parent()
        .expect("crossref dir has no parent — unexpected layout")
        .to_path_buf();

    println!("[layer2] repo root: {}", repo_root.display());
    println!(
        "[layer2] promote={} keep_up={} check_drift={} scenarios={:?}",
        flags.promote, flags.keep_up, flags.check_drift, flags.scenarios
    );

    // Stage soap-server BEFORE Topology::up.
    stage_soap_server(&repo_root);

    // Bring topology up (does down -v first for clean state).
    let topo = match Topology::up(&repo_root, flags.keep_up) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[layer2] topology up failed: {e}");
            std::process::exit(1);
        }
    };

    // Run all scenarios.
    let filter: Option<Vec<String>> = flags
        .scenarios
        .as_ref()
        .map(|s| s.split(',').map(|n| n.trim().to_string()).collect());
    let filter_slice = filter.as_deref();

    let report = run(
        &Endpoints::localhost(),
        &repo_root,
        flags.promote,
        filter_slice,
    );

    report.print();

    // ── Drift gate ──────────────────────────────────────────────────────────────
    let drift_exit_code: i32 = if flags.check_drift {
        let crossref_dir = repo_root.join("crossref");
        match drift::load(&crossref_dir) {
            Err(e) => {
                eprintln!("[drift] ERROR loading expected-failures.toml: {e}");
                1
            }
            Ok(baseline) => {
                let expected: std::collections::HashSet<String> =
                    baseline.into_iter().map(|ef| ef.scenario).collect();
                // Collect scenario names whose verdict is NOT Pass/KnownDivergence.
                let actual_failing: std::collections::HashSet<String> = report
                    .rows
                    .iter()
                    .filter(|(_, v)| {
                        !matches!(
                            v,
                            onvif_crossref::layer2::verdict::Verdict::Pass
                                | onvif_crossref::layer2::verdict::Verdict::KnownDivergence(_)
                        )
                    })
                    .map(|(name, _)| name.clone())
                    .collect();

                let result = drift::compute_drift(&actual_failing, &expected);
                result.print_summary();
                if result.is_clean() {
                    0
                } else {
                    1
                }
            }
        }
    } else {
        0
    };

    // Determine exit code before dropping topology.
    //
    // With --check-drift: gate solely on drift (clean baseline = exit 0, even if
    // known findings are red).  Without --check-drift: gate on absolute green
    // (any fail/error = exit 1).
    let exit_code = if flags.check_drift {
        drift_exit_code
    } else if report.is_green() {
        0i32
    } else {
        1i32
    };

    // Ensure topology is dropped (containers torn down) BEFORE exit.
    // Unless --keep-up was passed (Topology::up sets down_on_drop accordingly).
    drop(topo);

    std::process::exit(exit_code);
}

// ---------------------------------------------------------------------------
// Flag parsing
// ---------------------------------------------------------------------------

struct Flags {
    promote: bool,
    keep_up: bool,
    scenarios: Option<String>,
    check_drift: bool,
}

fn parse_flags(args: &[String]) -> Flags {
    let mut promote = false;
    let mut keep_up = false;
    let mut scenarios: Option<String> = None;
    let mut check_drift = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--promote" => promote = true,
            "--keep-up" => keep_up = true,
            "--check-drift" => check_drift = true,
            "--scenarios" => {
                i += 1;
                if i < args.len() {
                    scenarios = Some(args[i].clone());
                } else {
                    eprintln!("[layer2] --scenarios requires an argument");
                    std::process::exit(1);
                }
            }
            other => {
                eprintln!("[layer2] unknown flag: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    Flags {
        promote,
        keep_up,
        scenarios,
        check_drift,
    }
}

// ---------------------------------------------------------------------------
// Soap-server staging
// ---------------------------------------------------------------------------

/// Stage the sibling `soap-server` checkout into `<repo>/crossref/.build/soap-server/`
/// so the controlled-server Dockerfile can build it. The crossref crate's path
/// dependency is `../../soap-server` (i.e. a SIBLING of the repo root), so the source
/// is `<repo_root>/../soap-server`, not `<repo_root>/soap-server`.
fn stage_soap_server(repo_root: &std::path::Path) {
    let src = repo_root
        .parent()
        .expect("repo root has no parent — cannot locate sibling soap-server")
        .join("soap-server");
    let dest_parent = repo_root.join("crossref").join(".build");
    let dest = dest_parent.join("soap-server");

    // Ensure destination parent exists.
    if let Err(e) = std::fs::create_dir_all(&dest_parent) {
        eprintln!(
            "[layer2] stage: create_dir_all {}: {e}",
            dest_parent.display()
        );
    }

    // Try rsync first; fall back to cp -a.
    if try_rsync(&src, &dest) {
        println!("[layer2] staged soap-server via rsync → {}", dest.display());
    } else if try_cp(&src, &dest) {
        println!("[layer2] staged soap-server via cp -a → {}", dest.display());
    } else {
        eprintln!(
            "[layer2] WARNING: could not stage soap-server ({} → {}); build may fail",
            src.display(),
            dest.display()
        );
    }
}

fn try_rsync(src: &std::path::Path, dest: &std::path::Path) -> bool {
    let status = std::process::Command::new("rsync")
        .args([
            "-a",
            "--delete",
            "--exclude=target/",
            &format!("{}/", src.display()),
            &dest.to_string_lossy(),
        ])
        .status();
    matches!(status, Ok(s) if s.success())
}

fn try_cp(src: &std::path::Path, dest: &std::path::Path) -> bool {
    // Remove dest first to get a clean copy.
    let _ = std::fs::remove_dir_all(dest);
    let status = std::process::Command::new("cp")
        .args(["-a", &*src.to_string_lossy(), &*dest.to_string_lossy()])
        .status();
    matches!(status, Ok(s) if s.success())
}

// ---------------------------------------------------------------------------
// Unit tests for parse_flags
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn parse_flags_defaults() {
        let f = parse_flags(&[]);
        assert!(!f.promote);
        assert!(!f.keep_up);
        assert!(!f.check_drift);
        assert!(f.scenarios.is_none());
    }

    #[test]
    fn parse_flags_promote() {
        let f = parse_flags(&[s("--promote")]);
        assert!(f.promote);
        assert!(!f.keep_up);
    }

    #[test]
    fn parse_flags_keep_up() {
        let f = parse_flags(&[s("--keep-up")]);
        assert!(f.keep_up);
        assert!(!f.promote);
    }

    #[test]
    fn parse_flags_scenarios() {
        let f = parse_flags(&[
            s("--scenarios"),
            s("device_get_capabilities,device_get_services"),
        ]);
        assert_eq!(
            f.scenarios.as_deref(),
            Some("device_get_capabilities,device_get_services")
        );
    }

    #[test]
    fn parse_flags_check_drift() {
        let f = parse_flags(&[s("--check-drift")]);
        assert!(f.check_drift);
        assert!(!f.promote);
    }

    #[test]
    fn parse_flags_all_combined() {
        let f = parse_flags(&[
            s("--promote"),
            s("--keep-up"),
            s("--check-drift"),
            s("--scenarios"),
            s("foo"),
        ]);
        assert!(f.promote);
        assert!(f.keep_up);
        assert!(f.check_drift);
        assert_eq!(f.scenarios.as_deref(), Some("foo"));
    }
}
