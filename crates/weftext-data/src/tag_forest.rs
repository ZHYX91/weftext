//! Pure tag-forest validation and linear-size derived intervals. Core supplies
//! qualified bytes; this module never establishes identity or reads a file.
use crate::{DataError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TagDefinition {
    pub tag_id: String,
    pub display_name: String,
    // Explicit null is required. An omitted parent must not erase hierarchy.
    #[serde(deserialize_with = "deserialize_required_tag_parent")]
    pub parent_tag_id: Option<String>,
}
pub fn deserialize_required_tag_parent<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TagDocument {
    pub version: u32,
    pub tags: Vec<TagDefinition>,
}

#[derive(Debug, Clone)]
pub struct TagForest {
    intervals: BTreeMap<String, (usize, usize)>,
}
impl TagDocument {
    pub fn forest(&self) -> Result<TagForest> {
        if self.version != 2 || self.tags.len() > 5000 {
            return Err(DataError("Unsupported or oversized tag registry".into()));
        }
        let mut positions = BTreeMap::new();
        for (i, tag) in self.tags.iter().enumerate() {
            if !uuid::Uuid::parse_str(&tag.tag_id).is_ok_and(|id| {
                id.to_string() == tag.tag_id && id.get_version() == Some(uuid::Version::Random)
            }) || tag.display_name.trim().is_empty()
                || tag.display_name.chars().count() > 256
                || tag.display_name.contains('\0')
                || positions.insert(tag.tag_id.as_str(), i).is_some()
            {
                return Err(DataError("Invalid or duplicate tag declaration".into()));
            }
        }
        let mut children = vec![Vec::new(); self.tags.len()];
        let mut roots = Vec::new();
        for (i, tag) in self.tags.iter().enumerate() {
            match tag.parent_tag_id.as_deref() {
                None => roots.push(i),
                Some(parent) => {
                    let p = *positions
                        .get(parent)
                        .ok_or_else(|| DataError("Missing tag parent".into()))?;
                    if p == i {
                        return Err(DataError("A tag cannot parent itself".into()));
                    }
                    children[p].push(i);
                }
            }
        }
        let mut stack = roots
            .into_iter()
            .rev()
            .map(|i| (i, false))
            .collect::<Vec<_>>();
        let mut intervals = BTreeMap::new();
        let mut starts = vec![0; self.tags.len()];
        let mut clock = 0;
        while let Some((i, exiting)) = stack.pop() {
            if exiting {
                intervals.insert(self.tags[i].tag_id.clone(), (starts[i], clock));
            } else {
                starts[i] = clock;
                clock += 1;
                stack.push((i, true));
                stack.extend(children[i].iter().rev().map(|&c| (c, false)));
            }
        }
        if clock != self.tags.len() {
            return Err(DataError("Cyclic tag parent chain".into()));
        }
        Ok(TagForest { intervals })
    }
}
impl TagForest {
    pub fn contains(&self, id: &str) -> bool {
        self.intervals.contains_key(id)
    }
    pub fn in_subtree(&self, candidate: &str, root: &str) -> Result<bool> {
        let &(start, end) = self
            .intervals
            .get(root)
            .ok_or_else(|| DataError("Tag root unavailable".into()))?;
        let &(at, _) = self
            .intervals
            .get(candidate)
            .ok_or_else(|| DataError("Tag candidate unavailable".into()))?;
        Ok(start <= at && at < end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: usize) -> String {
        format!("00000000-0000-4000-8000-{n:012}")
    }
    #[test]
    fn long_chain_has_linear_storage_and_survives_reparenting() {
        let mut doc = TagDocument {
            version: 2,
            tags: (0..5000)
                .map(|i| TagDefinition {
                    tag_id: id(i),
                    display_name: "same/name".into(),
                    parent_tag_id: i.checked_sub(1).map(id),
                })
                .collect(),
        };
        let tree = doc.forest().unwrap();
        assert!(tree.in_subtree(&id(4999), &id(0)).unwrap());
        assert!(!tree.in_subtree(&id(0), &id(4999)).unwrap());
        assert_eq!(tree.intervals.len(), 5000);
        doc.tags[2500].parent_tag_id = None;
        assert!(!doc.forest().unwrap().in_subtree(&id(4999), &id(0)).unwrap());
        doc.tags[0].parent_tag_id = Some(id(2499));
        assert!(doc.forest().is_err());
    }
    #[test]
    fn missing_parent_is_not_an_implicit_root() {
        assert!(
            serde_json::from_value::<TagDefinition>(
                serde_json::json!({"tagId":id(0),"displayName":"a"})
            )
            .is_err()
        );
        let doc = TagDocument {
            version: 2,
            tags: vec![TagDefinition {
                tag_id: id(0),
                display_name: "a".into(),
                parent_tag_id: Some(id(1)),
            }],
        };
        assert!(doc.forest().is_err());
    }
}
