//! onvif-crossref — differential conformance & interop harness for onvif-server.
//! Phase 2a: Layer-1 replay/diff against `unverified` baselines (no Docker).
pub mod fixture;
pub mod invariants;
pub mod masks;
pub mod normalize;
pub mod scenario;
pub mod snapshot;
pub mod sut;
