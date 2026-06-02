//! onvif-crossref — differential conformance & interop harness for onvif-server.
//! Phase 2a: Layer-1 replay/diff against `unverified` baselines (no Docker).
//! Phase 2b: Layer-2 Docker conformance pipeline.
pub mod body_extract;
pub mod fixture;
pub mod invariants;
pub mod layer2;
pub mod masks;
pub mod normalize;
pub mod oracle;
pub mod projection;
pub mod scenario;
pub mod snapshot;
pub mod sut;
