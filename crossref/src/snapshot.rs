//! Golden snapshot store with provenance status (spec §5.2).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Unverified,
    Verified,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Unverified => "unverified",
            Status::Verified => "verified",
        }
    }
    fn parse(s: &str) -> Option<Status> {
        match s {
            "unverified" => Some(Status::Unverified),
            "verified" => Some(Status::Verified),
            _ => None,
        }
    }
}

pub struct SnapshotStore {
    dir: PathBuf,
}

impl SnapshotStore {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        SnapshotStore {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    fn snap_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.xml"))
    }
    fn status_path(&self) -> PathBuf {
        self.dir.join("status.toml")
    }

    fn load_status_map(&self) -> BTreeMap<String, String> {
        match std::fs::read_to_string(self.status_path()) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => BTreeMap::new(),
        }
    }
    fn save_status_map(&self, map: &BTreeMap<String, String>) -> Result<(), String> {
        let s = toml::to_string_pretty(map).map_err(|e| e.to_string())?;
        std::fs::write(self.status_path(), s).map_err(|e| e.to_string())
    }

    pub fn read(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.snap_path(name)).ok()
    }

    pub fn status(&self, name: &str) -> Option<Status> {
        self.load_status_map()
            .get(name)
            .and_then(|s| Status::parse(s))
    }

    pub fn write_unverified(&self, name: &str, normalized: &str) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        std::fs::write(self.snap_path(name), normalized).map_err(|e| e.to_string())?;
        let mut map = self.load_status_map();
        map.insert(name.to_string(), Status::Unverified.as_str().to_string());
        self.save_status_map(&map)
    }

    pub fn unverified_count(&self) -> Result<usize, String> {
        Ok(self
            .load_status_map()
            .values()
            .filter(|v| v.as_str() == Status::Unverified.as_str())
            .count())
    }

    /// Flip a scenario's status to `verified` in status.toml WITHOUT touching the
    /// Layer-1 snapshot bytes. Used by Layer-2 promotion (spec §5.2).
    pub fn write_verified(&self, name: &str) -> Result<(), String> {
        let mut map = self.load_status_map();
        map.insert(name.to_string(), Status::Verified.as_str().to_string());
        self.save_status_map(&map)
    }

    /// Store the oracle-canonical conformance evidence bytes under
    /// `snapshots/canonical/<name>.c14n`. Layer-1 snapshot bytes are NOT touched.
    pub fn write_canonical(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let canon_dir = self.dir.join("canonical");
        std::fs::create_dir_all(&canon_dir).map_err(|e| e.to_string())?;
        std::fs::write(canon_dir.join(format!("{name}.c14n")), bytes).map_err(|e| e.to_string())
    }

    /// Read the oracle-canonical evidence bytes for a scenario (if promoted).
    pub fn read_canonical(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.dir.join("canonical").join(format!("{name}.c14n"))).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        // Include thread id to avoid collision when tests run in parallel (all share process id).
        p.push(format!(
            "crossref-snap-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn writes_unverified_and_reads_back() {
        let dir = tmp();
        let store = SnapshotStore::new(&dir);
        store.write_unverified("op_x", "<normalized/>").unwrap();
        assert_eq!(store.read("op_x").unwrap(), "<normalized/>");
        assert_eq!(store.status("op_x").unwrap(), Status::Unverified);
    }

    #[test]
    fn missing_snapshot_reads_none() {
        let dir = tmp();
        let store = SnapshotStore::new(&dir);
        assert!(store.read("absent").is_none());
    }

    #[test]
    fn counts_unverified() {
        let dir = tmp();
        let store = SnapshotStore::new(&dir);
        store.write_unverified("a", "<a/>").unwrap();
        store.write_unverified("b", "<b/>").unwrap();
        assert_eq!(store.unverified_count().unwrap(), 2);
    }

    #[test]
    fn write_verified_flips_status_leaves_snapshot_bytes_intact() {
        let dir = tmp();
        let store = SnapshotStore::new(&dir);
        // First capture an unverified snapshot.
        store.write_unverified("sc_a", "<original/>").unwrap();
        assert_eq!(store.status("sc_a").unwrap(), Status::Unverified);
        // Promote to verified — must not touch the .xml bytes.
        store.write_verified("sc_a").unwrap();
        assert_eq!(store.status("sc_a").unwrap(), Status::Verified);
        // .xml bytes unchanged.
        assert_eq!(store.read("sc_a").unwrap(), "<original/>");
    }

    #[test]
    fn write_canonical_round_trip() {
        let dir = tmp();
        let store = SnapshotStore::new(&dir);
        let bytes = b"<canonical>evidence</canonical>";
        store.write_canonical("sc_b", bytes).unwrap();
        let back = store.read_canonical("sc_b").unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn write_verified_does_not_decrement_unverified_for_new_entry() {
        let dir = tmp();
        let store = SnapshotStore::new(&dir);
        // write_verified on a name never write_unverified-d (promotes a fresh name).
        store.write_verified("fresh").unwrap();
        assert_eq!(store.status("fresh").unwrap(), Status::Verified);
        assert_eq!(store.unverified_count().unwrap(), 0);
    }
}
