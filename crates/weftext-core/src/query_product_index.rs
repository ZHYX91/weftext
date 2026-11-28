//! Rebuildable native Query projections. A cache hit consumes the same newly
//! qualified source, style and dependency inputs as a fresh compilation.
use crate::{Core, ParentGuard, backend, hash};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
};
use weftext_engine::{Dependency, FrozenQueryProduct, RenderProducts, StyleModule};

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 6000;
const FORMAT: &str = "weftext.native-query-products.v1";
const INDEX_FILE: &str = "query-products.json";

pub struct QueryProductCut {
    files: BTreeMap<String, String>,
    identity: String,
    style: String,
}

pub struct QueryProductKey {
    slot: String,
    basis: String,
}

impl QueryProductCut {
    pub fn new(
        partition: &[&str],
        files: &BTreeMap<String, Vec<u8>>,
        style: Option<&StyleModule>,
    ) -> Self {
        let identity = digest(&(FORMAT, partition));
        let files = files
            .iter()
            .map(|(path, bytes)| (path.replace('\\', "/"), hash(bytes)))
            .collect();
        Self {
            files,
            identity,
            style: digest(&style),
        }
    }

    pub fn key(&self, entity: &str, path: &str, source: &str) -> QueryProductKey {
        QueryProductKey {
            slot: digest(&(&self.identity, entity)),
            basis: digest(&(&self.style, path, hash(source.as_bytes()))),
        }
    }

    pub fn file_fingerprint(&self, path: &str) -> Option<&str> {
        self.files.get(&path.replace('\\', "/")).map(String::as_str)
    }

    pub fn record(
        &self,
        key: &QueryProductKey,
        product: &FrozenQueryProduct,
    ) -> Option<IndexedQueryProduct> {
        let products = product
            .products
            .as_ref()
            .filter(|products| products.target == "html")?;
        let mut witness = BTreeMap::new();
        for dependency in &product.dependencies {
            // The compiler emits these only for its own wrapper over the
            // exact main source and style selection in this key.
            if matches!(dependency.kind.as_str(), "derived-source" | "derived-file") {
                if dependency.status != "loaded" {
                    return None;
                }
                continue;
            }
            if !matches!(dependency.kind.as_str(), "source" | "file")
                || dependency.status != "loaded"
            {
                return None;
            }
            let name = dependency.path.replace('\\', "/");
            witness.insert(name.clone(), self.files.get(&name)?.clone());
        }
        Some(IndexedQueryProduct {
            key: key.slot.clone(),
            basis: key.basis.clone(),
            witness,
            products: products.clone(),
            dependencies: product.dependencies.clone(),
            compiler_version: product.compiler_version.clone(),
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedQueryProduct {
    key: String,
    basis: String,
    witness: BTreeMap<String, String>,
    pub products: RenderProducts,
    pub dependencies: Vec<Dependency>,
    pub compiler_version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexFile<Entries> {
    format: String,
    checksum: String,
    entries: Entries,
}

#[derive(Default)]
pub(crate) struct QueryProductIndex {
    entries: BTreeMap<String, (usize, IndexedQueryProduct)>,
    order: VecDeque<String>,
    bytes: usize,
    dirty: bool,
}

fn digest(value: &impl Serialize) -> String {
    hash(&serde_json::to_vec(value).expect("native Query index identity serializes"))
}

impl QueryProductIndex {
    fn load(core: &Core) -> Self {
        let read = || -> Option<Self> {
            let path = core.data_dir.join(INDEX_FILE);
            let guard = ParentGuard::acquire(&core.backend, &path).ok()?;
            let file = guard.open_file(false, false).ok()?;
            let mut bytes = Vec::new();
            file.take((MAX_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() > MAX_BYTES {
                return None;
            }
            let index: IndexFile<Vec<IndexedQueryProduct>> = serde_json::from_slice(&bytes).ok()?;
            if index.format != FORMAT
                || index.entries.len() > MAX_ENTRIES
                || digest(&index.entries) != index.checksum
            {
                return None;
            }
            let mut store = Self::default();
            for entry in index.entries {
                store.insert(entry);
            }
            store.dirty = false;
            Some(store)
        };
        read().unwrap_or_default()
    }

    fn insert(&mut self, entry: IndexedQueryProduct) {
        let size = serde_json::to_vec(&entry)
            .expect("native Query product serializes")
            .len()
            + 1;
        if size > MAX_BYTES - 1024 {
            return;
        }
        let key = entry.key.clone();
        if let Some((bytes, _)) = self.entries.remove(&key) {
            self.bytes -= bytes;
            self.order.retain(|candidate| candidate != &key);
        }
        while size > MAX_BYTES - 1024 - self.bytes || self.entries.len() >= MAX_ENTRIES {
            let oldest = self.order.pop_front().expect("ordered native Query entry");
            self.bytes -= self.entries.remove(&oldest).expect("native Query entry").0;
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, (size, entry));
        self.bytes += size;
        self.dirty = true;
    }

    fn persist(&mut self, core: &Core) -> crate::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let entries = self
            .order
            .iter()
            .map(|key| &self.entries[key].1)
            .collect::<Vec<_>>();
        let record = IndexFile {
            format: FORMAT.into(),
            checksum: digest(&entries),
            entries,
        };
        let bytes = serde_json::to_vec(&record)?;
        let temporary = core
            .data_dir
            .join(format!(".query-products.{}.pending", crate::fresh()));
        let destination = core.data_dir.join(INDEX_FILE);
        let guard = ParentGuard::acquire(&core.backend, &temporary)?;
        let mut file = guard.open_file(true, true)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        backend::publish_private_replace(&core.backend, &temporary, &destination)?;
        self.dirty = false;
        Ok(())
    }
}

impl Core {
    pub fn query_product_entries<'cut>(
        &mut self,
        inputs: impl IntoIterator<Item = (&'cut QueryProductCut, &'cut QueryProductKey)>,
    ) -> Vec<Option<IndexedQueryProduct>> {
        if self.query_product_index.is_none() {
            self.query_product_index = Some(QueryProductIndex::load(self));
        }
        let store = self.query_product_index.as_ref().unwrap();
        inputs
            .into_iter()
            .map(|(cut, key)| {
                let (_, entry) = store.entries.get(&key.slot)?;
                (entry.basis == key.basis
                    && entry
                        .witness
                        .iter()
                        .all(|(path, hash)| cut.files.get(path) == Some(hash)))
                .then(|| entry.clone())
            })
            .collect()
    }

    pub fn remember_query_products(&mut self, entries: Vec<IndexedQueryProduct>) {
        let mut store = self
            .query_product_index
            .take()
            .unwrap_or_else(|| QueryProductIndex::load(self));
        for entry in entries {
            store.insert(entry);
        }
        // The projections are disposable. Publishing failure cannot
        // invalidate the original authorized compilation or its result.
        let _ = store.persist(self);
        self.query_product_index = Some(store);
    }
}
