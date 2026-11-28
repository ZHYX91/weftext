use super::{
    CelBool, CelString, CowVal, DataError, ExecutionError, HostFailures, Result, UiMessage,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

const MAX_TEXT_BYTES: usize = 512 * 1024;
const MAX_TERMS_BYTES: usize = 4096;
const MAX_TERMS: usize = 64;
const MAX_TOKENS: usize = 64 * 1024;
const MAX_CACHE_BYTES: usize = 4 * 1024 * 1024;

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
        let mut retained_bytes = text.len();
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

#[derive(Default)]
pub(super) struct TextIndexes {
    indexes: BTreeMap<String, TextIndex>,
    terms: BTreeMap<String, Vec<String>>,
    retained_bytes: usize,
}

impl TextIndexes {
    fn find(&mut self, text: &str, query: &str, mode: &str) -> Result<bool> {
        if query.len() > MAX_TERMS_BYTES {
            return Err(budget());
        }
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
        if let Some(index) = self.indexes.get(text) {
            return Ok(index.matches(terms, mode));
        }
        let index = TextIndex::build(text)?;
        let matched = index.matches(terms, mode);
        if self.indexes.len() < 512
            && index.retained_bytes <= MAX_CACHE_BYTES.saturating_sub(self.retained_bytes)
        {
            self.retained_bytes += index.retained_bytes;
            self.indexes.insert(text.into(), index);
        }
        Ok(matched)
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
        assert_eq!(query.indexes.len(), 1);
    }
}
