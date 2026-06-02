//! Rust client for the containerised Java XML oracle (validate + exclusive C14N).
//! The orchestrator NEVER validates or canonicalizes XML itself — it always delegates
//! to this oracle (spec §4.3).

pub struct Oracle {
    base: String,
    http: reqwest::blocking::Client,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

impl Oracle {
    pub fn new(base: impl Into<String>) -> Self {
        Oracle {
            base: base.into(),
            http: reqwest::blocking::Client::new(),
        }
    }

    /// Exclusive-C14N the given XML bytes. Returns canonical bytes.
    pub fn c14n(&self, xml: &[u8]) -> Result<Vec<u8>, String> {
        let r = self
            .http
            .post(format!("{}/c14n", self.base))
            .body(xml.to_vec())
            .send()
            .map_err(|e| e.to_string())?;
        if !r.status().is_success() {
            return Err(format!(
                "c14n {}: {}",
                r.status(),
                r.text().unwrap_or_default()
            ));
        }
        Ok(r.bytes().map_err(|e| e.to_string())?.to_vec())
    }

    /// Validate `xml` against the named schema id.
    pub fn validate(&self, xml: &[u8], schema: &str) -> Result<ValidationResult, String> {
        let r = self
            .http
            .post(format!("{}/validate?schema={}", self.base, schema))
            .body(xml.to_vec())
            .send()
            .map_err(|e| e.to_string())?;
        let v: serde_json::Value = r.json().map_err(|e| e.to_string())?;
        Ok(ValidationResult {
            valid: v.get("valid").and_then(|b| b.as_bool()).unwrap_or(false),
            errors: v
                .get("errors")
                .and_then(|e| e.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Spin a minimal hand-rolled HTTP/1.1 stub on an ephemeral port.
    /// Reads one request (enough to drain headers + body), then writes `response`.
    /// Returns the port. The thread exits after one request.
    fn stub_once(response: &'static [u8]) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            // Drain the incoming request (headers + blank line + body).
            let mut buf = [0u8; 4096];
            let mut raw = Vec::new();
            loop {
                let n = stream.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
                // Stop once we have a blank line separating headers from body.
                if raw.windows(4).any(|w| w == b"\r\n\r\n") {
                    // Also need to read Content-Length bytes of body if any.
                    // Parse Content-Length from raw so we read the full body.
                    let header_end = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
                    let header_str = std::str::from_utf8(&raw[..header_end]).unwrap_or("");
                    let content_len: usize = header_str
                        .lines()
                        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                        .and_then(|l| l.split(':').nth(1))
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(0);
                    let body_have = raw.len() - header_end;
                    let remaining = content_len.saturating_sub(body_have);
                    let mut extra = vec![0u8; remaining];
                    stream.read_exact(&mut extra).unwrap_or(());
                    break;
                }
            }
            stream.write_all(response).unwrap_or(());
        });
        port
    }

    #[test]
    fn validate_invalid_parses_errors() {
        let body = b"{\"valid\":false,\"errors\":[\"boom\"]}";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );
        let resp_bytes: &'static [u8] = Box::leak(resp.into_bytes().into_boxed_slice());
        let port = stub_once(resp_bytes);

        let oracle = Oracle::new(format!("http://127.0.0.1:{port}"));
        let result = oracle
            .validate(b"<x/>", "soap12-envelope")
            .expect("validate call");
        assert!(!result.valid);
        assert_eq!(result.errors, vec!["boom".to_string()]);
    }

    #[test]
    fn validate_valid_parses_empty_errors() {
        let body = b"{\"valid\":true}";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );
        let resp_bytes: &'static [u8] = Box::leak(resp.into_bytes().into_boxed_slice());
        let port = stub_once(resp_bytes);

        let oracle = Oracle::new(format!("http://127.0.0.1:{port}"));
        let result = oracle
            .validate(b"<x/>", "soap12-envelope")
            .expect("validate call");
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn c14n_returns_body_verbatim() {
        let canonical = b"<a a=\"1\" b=\"2\">x</a>";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n\r\n",
            canonical.len()
        );
        let mut full = resp.into_bytes();
        full.extend_from_slice(canonical);
        let resp_bytes: &'static [u8] = Box::leak(full.into_boxed_slice());
        let port = stub_once(resp_bytes);

        let oracle = Oracle::new(format!("http://127.0.0.1:{port}"));
        let result = oracle
            .c14n(b"<a   b=\"2\" a=\"1\">x</a>")
            .expect("c14n call");
        assert_eq!(result, canonical.to_vec());
    }
}
