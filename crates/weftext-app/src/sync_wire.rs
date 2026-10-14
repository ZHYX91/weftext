//! One bounded wire vocabulary for local clients and hosted owner transport.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use weftext_core::sync::SyncBundle;
pub const CHUNK: usize = 65_536;
pub const MAX_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_RECORDS: usize = 22_000;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub manifest: bool,
    pub id: String,
    pub bytes: usize,
}
impl Record {
    pub fn validate(&self) -> bool {
        self.id.len() == 64
            && self
                .id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && self.bytes
                <= if self.manifest {
                    4 * 1024 * 1024
                } else {
                    64 * 1024 * 1024
                }
    }
}
pub fn records(bundle: SyncBundle) -> Result<Vec<(Record, Vec<u8>)>, serde_json::Error> {
    let mut records = Vec::new();
    for (id, bytes) in bundle.blobs {
        records.push((
            Record {
                manifest: false,
                id,
                bytes: bytes.len(),
            },
            bytes,
        ));
    }
    for (id, manifest) in bundle.manifests {
        let bytes = serde_json::to_vec(&manifest)?;
        records.push((
            Record {
                manifest: true,
                id,
                bytes: bytes.len(),
            },
            bytes,
        ));
    }
    Ok(records)
}
pub fn check_catalog(records: &[Record]) -> bool {
    let mut keys = BTreeMap::new();
    records.len() <= MAX_RECORDS
        && records
            .iter()
            .all(|r| r.validate() && keys.insert((r.manifest, &r.id), ()).is_none())
        && records.iter().map(|r| r.bytes as u64).sum::<u64>() <= MAX_BYTES as u64
}
