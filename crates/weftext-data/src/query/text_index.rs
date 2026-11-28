use super::{
    CelBool, CelString, CowVal, DataError, ExecutionError, HostFailures, Result, UiMessage,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

const MAX_TEXT_BYTES: usize = 512 * 1024;
const MAX_TERMS_BYTES: usize = 4096;
const MAX_TERMS: usize = 64;
const MAX_TOKENS: usize = 64 * 1024;
const MAX_CACHE_BYTES: usize = 16 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 4096;
const MAX_TEXT_WORK_BYTES: usize = 64 * 1024 * 1024;

fn budget() -> DataError {
    DataError::budget(UiMessage::error_data_query_text_index_budget())
}

pub(super) fn valid_mode(mode: &str) -> bool {
    matches!(mode, "all" | "any" | "phrase" | "exclude")
}

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.unicode_words()
        .map(|word| word.nfkc().flat_map(char::to_lowercase).collect())
}

#[derive(Debug)]
struct TextIndex {
    postings: BTreeMap<String, Vec<usize>>,
    retained_bytes: usize,
}

impl TextIndex {
    fn build(text: &str) -> Result<Self> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(budget());
        }
        let mut postings = BTreeMap::<String, Vec<usize>>::new();
        let mut retained_bytes = 256;
        for (position, word) in words(text).enumerate() {
            if position >= MAX_TOKENS {
                return Err(budget());
            }
            let bytes = word.len();
            let locations = postings.entry(word).or_default();
            if locations.is_empty() {
                retained_bytes += bytes + 112;
            }
            retained_bytes += 2 * std::mem::size_of::<usize>();
            locations.push(position);
        }
        Ok(Self {
            postings,
            retained_bytes,
        })
    }

    fn matches(&self, terms: &[String], mode: &str) -> bool {
        if terms.is_empty() {
            return false;
        }
        match mode {
            "all" => terms.iter().all(|term| self.postings.contains_key(term)),
            "any" => terms.iter().any(|term| self.postings.contains_key(term)),
            "exclude" => terms.iter().all(|term| !self.postings.contains_key(term)),
            "phrase" => {
                let lists: Option<Vec<_>> =
                    terms.iter().map(|term| self.postings.get(term)).collect();
                let Some(lists) = lists else { return false };
                let (pivot, positions) = lists
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, positions)| positions.len())
                    .unwrap();
                positions.iter().any(|position| {
                    position.checked_sub(pivot).is_some_and(|start| {
                        lists.iter().enumerate().all(|(index, positions)| {
                            positions.binary_search(&(start + index)).is_ok()
                        })
                    })
                })
            }
            _ => false,
        }
    }
}

type IndexKey = ([u8; 32], [u8; 32]);

#[derive(Debug, Default)]
struct IndexStore {
    entries: BTreeMap<IndexKey, Arc<TextIndex>>,
    insertion_order: VecDeque<IndexKey>,
    retained_bytes: usize,
}

/// Bounded, disposable postings for already-qualified text. The handle cannot
/// enumerate text or return search hits independently of the current input.
#[derive(Clone, Default)]
pub struct QueryTextIndex {
    store: Arc<Mutex<IndexStore>>,
    scope: [u8; 32],
}

impl std::fmt::Debug for QueryTextIndex {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QueryTextIndex")
            .finish_non_exhaustive()
    }
}

impl QueryTextIndex {
    pub fn scoped(&self, parts: &[&str]) -> Self {
        let mut hash = Sha256::new();
        for part in parts {
            hash.update((part.len() as u64).to_le_bytes());
            hash.update(part.as_bytes());
        }
        Self {
            store: self.store.clone(),
            scope: hash.finalize().into(),
        }
    }

    fn index(&self, text: &str) -> Result<Arc<TextIndex>> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(budget());
        }
        let key = (self.scope, Sha256::digest(text.as_bytes()).into());
        if let Some(index) = self
            .store
            .lock()
            .expect("text index store")
            .entries
            .get(&key)
        {
            return Ok(index.clone());
        }
        // Concurrent misses may build the same value, but the store lock is
        // never held during tokenization or phrase matching.
        let index = Arc::new(TextIndex::build(text)?);
        let mut store = self.store.lock().expect("text index store");
        if let Some(index) = store.entries.get(&key) {
            return Ok(index.clone());
        }
        if index.retained_bytes <= MAX_CACHE_BYTES {
            while store.entries.len() >= MAX_CACHE_ENTRIES
                || index.retained_bytes > MAX_CACHE_BYTES - store.retained_bytes
            {
                let oldest = store.insertion_order.pop_front().expect("indexed entry");
                let evicted = store.entries.remove(&oldest).expect("indexed entry");
                store.retained_bytes -= evicted.retained_bytes;
            }
            store.retained_bytes += index.retained_bytes;
            store.insertion_order.push_back(key);
            store.entries.insert(key, index.clone());
        }
        Ok(index)
    }
}

#[derive(Default)]
pub(super) struct TextIndexes {
    pub cache: QueryTextIndex,
    terms: BTreeMap<String, Vec<String>>,
    touched_bytes: usize,
}

impl TextIndexes {
    fn find(&mut self, text: &str, query: &str, mode: &str) -> Result<bool> {
        let charge = text.len().saturating_add(query.len());
        if query.len() > MAX_TERMS_BYTES || charge > MAX_TEXT_WORK_BYTES - self.touched_bytes {
            return Err(budget());
        }
        self.touched_bytes += charge;
        let unretained = if self.terms.contains_key(query) {
            None
        } else {
            let parsed = words(query).take(MAX_TERMS + 1).collect::<Vec<_>>();
            if parsed.len() > MAX_TERMS {
                return Err(budget());
            }
            if self.terms.len() < 64 {
                self.terms.insert(query.into(), parsed);
                None
            } else {
                Some(parsed)
            }
        };
        let terms = self
            .terms
            .get(query)
            .or(unretained.as_ref())
            .expect("parsed query terms");
        Ok(self.cache.index(text)?.matches(terms, mode))
    }
}

pub(super) struct TextMatchFunction {
    pub indexes: Arc<Mutex<TextIndexes>>,
    pub failures: HostFailures,
}

impl cel::extractors::IntoFunction<TextMatchFunction> for TextMatchFunction {
    fn into_function(
        self,
    ) -> Box<
        dyn for<'c, 'v> Fn(
                &mut cel::FunctionContext<'c, 'v>,
            ) -> std::result::Result<CowVal<'c, 'v>, ExecutionError>
            + Send
            + Sync,
    > {
        Box::new(move |context| {
            let text = |index: usize| {
                context.args.get(index).and_then(|value| {
                    value
                        .as_ref()
                        .downcast_ref::<CelString>()
                        .map(|string| string.inner())
                })
            };
            let result = match (text(0), text(1), text(2)) {
                (Some(content), Some(query), Some(mode)) if valid_mode(mode) => self
                    .indexes
                    .lock()
                    .expect("query-local text index")
                    .find(content, query, mode),
                _ => Err(DataError::new(
                    UiMessage::error_data_query_text_index_arguments(),
                )),
            };
            result
                .map(|matched| CowVal::Owned(Box::new(CelBool::from(matched))))
                .map_err(|error| self.failures.error(error))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_index_normalizes_text_and_distinguishes_ordered_phrases() {
        let mut query = TextIndexes::default();
        let body = "Ｆｉｎａｌ 会议 notes and Café";
        assert!(query.find(body, "cafe\u{301} final", "all").unwrap());
        assert!(query.find(body, "final 会议", "phrase").unwrap());
        assert!(!query.find(body, "会议 final", "phrase").unwrap());
        assert!(query.find(body, "报告 notes", "any").unwrap());
        assert!(query.find(body, "absent terms", "exclude").unwrap());
        assert!(!query.find(body, "note", "all").unwrap());
        assert_eq!(query.cache.store.lock().unwrap().entries.len(), 1);
    }

    #[test]
    fn scoped_cache_reuses_exact_content_and_isolates_changed_authority() {
        let root = QueryTextIndex::default();
        let before = root.scoped(&["actor-a", "workspace", "1"]);
        let old = before.index("original body").unwrap();
        let read_again = root.scoped(&["actor-a", "workspace", "1"]);
        assert!(Arc::ptr_eq(
            &old,
            &read_again.index("original body").unwrap()
        ));
        let changed = read_again.index("updated body").unwrap();
        assert!(!changed.matches(&["original".into()], "all"));
        assert!(changed.matches(&["updated".into()], "all"));
        for parts in [
            ["actor-b", "workspace", "1"],
            ["actor-a", "workspace", "2"],
            ["actor-a", "other-workspace", "1"],
        ] {
            assert!(!Arc::ptr_eq(
                &old,
                &root.scoped(&parts).index("original body").unwrap()
            ));
        }
        assert_eq!(root.store.lock().unwrap().entries.len(), 5);
        assert!(root.index(&"a".repeat(MAX_TEXT_BYTES + 1)).is_err());
    }

    #[test]
    fn concurrent_builds_publish_one_complete_entry() {
        let cache = QueryTextIndex::default();
        let gate = std::sync::Barrier::new(4);
        let indexes = std::thread::scope(|scope| {
            let workers = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        gate.wait();
                        cache.index("first next last").unwrap()
                    })
                })
                .collect::<Vec<_>>();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(indexes.iter().all(|index| Arc::ptr_eq(index, &indexes[0])));
        assert!(indexes[0].matches(&["next".into(), "last".into()], "phrase"));
        let store = cache.store.lock().unwrap();
        assert_eq!(store.entries.len(), 1);
        assert_eq!(store.retained_bytes, indexes[0].retained_bytes);
    }
}
