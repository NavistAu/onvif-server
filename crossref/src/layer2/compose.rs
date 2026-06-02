//! Layer-2 compose lifecycle: up (build) → wait healthy → down. Shells out to the
//! `docker` CLI to avoid a Docker SDK dependency in the published-adjacent crate.

use std::path::Path;
use std::process::Command;

const COMPOSE_FILE: &str = "crossref/docker-compose.yml";
const COMPOSE_LOCAL: &str = "crossref/docker-compose.local.yml";

pub struct Topology {
    repo_root: std::path::PathBuf,
    down_on_drop: bool,
}

impl Topology {
    /// `docker compose up -d --build` (both -f files), then block until all services are healthy.
    ///
    /// Runs `down -v` first (ignoring errors) so every run starts from a guaranteed-clean slate
    /// regardless of prior leaks (belt: teardown before up; suspenders: Drop runs after exit).
    pub fn up(repo_root: &Path, keep_up: bool) -> Result<Self, String> {
        // Belt: unconditionally tear down any leftover containers/volumes before starting.
        // Ignore the error — nothing may be running, which is fine.
        let _ = run(
            repo_root,
            &[
                "compose",
                "-f",
                COMPOSE_FILE,
                "-f",
                COMPOSE_LOCAL,
                "down",
                "-v",
            ],
        );
        run(
            repo_root,
            &[
                "compose",
                "-f",
                COMPOSE_FILE,
                "-f",
                COMPOSE_LOCAL,
                "up",
                "-d",
                "--build",
            ],
        )?;
        wait_healthy(
            repo_root,
            &["controlled-server", "oracle", "onvif-srvd"],
            180,
        )?;
        Ok(Topology {
            repo_root: repo_root.to_path_buf(),
            down_on_drop: !keep_up,
        })
    }

    pub fn down(repo_root: &Path) -> Result<(), String> {
        run(
            repo_root,
            &[
                "compose",
                "-f",
                COMPOSE_FILE,
                "-f",
                COMPOSE_LOCAL,
                "down",
                "-v",
            ],
        )
    }
}

impl Drop for Topology {
    fn drop(&mut self) {
        if self.down_on_drop {
            let _ = Topology::down(&self.repo_root);
        }
    }
}

fn run(dir: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("docker")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("docker: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "docker {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

fn wait_healthy(dir: &Path, services: &[&str], max_secs: u64) -> Result<(), String> {
    use std::time::{Duration, Instant};
    let start = Instant::now();
    loop {
        let mut all = true;
        for s in services {
            let out = Command::new("docker")
                .args([
                    "compose",
                    "-f",
                    COMPOSE_FILE,
                    "-f",
                    COMPOSE_LOCAL,
                    "ps",
                    "-q",
                    s,
                ])
                .current_dir(dir)
                .output()
                .map_err(|e| e.to_string())?;
            let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if id.is_empty() {
                all = false;
                break;
            }
            let h = Command::new("docker")
                .args(["inspect", "-f", "{{.State.Health.Status}}", &id])
                .output()
                .map_err(|e| e.to_string())?;
            if String::from_utf8_lossy(&h.stdout).trim() != "healthy" {
                all = false;
                break;
            }
        }
        if all {
            return Ok(());
        }
        if start.elapsed() > Duration::from_secs(max_secs) {
            return Err("topology did not become healthy in time".into());
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}
