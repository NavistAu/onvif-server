//! Snapshot promotion (spec §5.2): on a Pass verdict, store oracle-canonical bytes as
//! conformance evidence and flip the scenario's status to `verified`.
//! Layer-1 snapshot bytes (the `.xml` files) are NOT overwritten.

use crate::snapshot::SnapshotStore;

/// Promote a scenario to `verified`:
/// 1. Write oracle-canonical bytes as `snapshots/canonical/<name>.c14n`.
/// 2. Flip the status entry in `status.toml` to `"verified"`.
///
/// The Layer-1 `.xml` snapshot is deliberately left intact so Layer-1 replay
/// continues to diff against its own self-captured baseline (spec §5.2 reconciliation).
pub fn promote(store: &SnapshotStore, name: &str, canonical: &[u8]) -> Result<(), String> {
    store.write_canonical(name, canonical)?;
    store.write_verified(name)
}
