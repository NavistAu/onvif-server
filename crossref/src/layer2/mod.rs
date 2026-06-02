//! Layer-2 orchestration: compose lifecycle, endpoints, verdict, promotion, report.

pub mod compose;
pub mod promote;
pub mod report;
pub mod verdict;

use report::Report;
pub use verdict::{Eval, Resp, Verdict};

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
            srvd: "http://localhost:8083".into(),
            oracle: "http://localhost:8081".into(),
        }
    }
}

/// Drive all in-scope ONVIF conformance scenarios, return the verdict report.
///
/// `scenarios_filter`: if `Some`, only run the listed scenario names.
///
/// NOTE: This function is stubbed pending Phase 2b Task 6 which writes the
/// onvif-specific scenario-driving logic (dynamic WS-Security injection, per-service
/// routing, srvd comparison modes, invariant evaluation).
pub fn run(
    _endpoints: &Endpoints,
    _repo_root: &std::path::Path,
    _promote_on_pass: bool,
    _scenarios_filter: Option<&[String]>,
) -> Report {
    unimplemented!("Phase 2b Task 6")
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
