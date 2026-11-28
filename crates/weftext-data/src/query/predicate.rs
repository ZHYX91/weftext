use super::{DataError, QueryPlan, Result, UiMessage, parse_cel};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

const MAX_DEPTH: usize = 12;
const MAX_NODES: usize = 256;
const MAX_CEL_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryPredicate {
    Expression(String),
    Group(QueryPredicateGroup),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryPredicateGroup {
    pub op: QueryBoolean,
    pub conditions: Vec<QueryPredicate>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryBoolean {
    All,
    Any,
    Not,
}

impl From<&str> for QueryPredicate {
    fn from(source: &str) -> Self {
        Self::Expression(source.into())
    }
}

impl From<String> for QueryPredicate {
    fn from(source: String) -> Self {
        Self::Expression(source)
    }
}

impl QueryPlan {
    pub fn predicate_source(&self) -> Result<Option<Cow<'_, str>>> {
        self.r#where.as_ref().map(QueryPredicate::cel).transpose()
    }
}

impl QueryPredicate {
    pub fn cel(&self) -> Result<Cow<'_, str>> {
        if let Self::Expression(source) = self {
            return Ok(Cow::Borrowed(source));
        }
        let mut source = String::new();
        self.write_cel(&mut source, 0, &mut 0)?;
        Ok(Cow::Owned(source))
    }

    fn write_cel(&self, source: &mut String, depth: usize, nodes: &mut usize) -> Result<()> {
        *nodes += 1;
        let budget = || DataError::budget(UiMessage::error_data_query_condition_group_budget());
        if depth > MAX_DEPTH || *nodes > MAX_NODES {
            return Err(budget());
        }
        source.push('(');
        match self {
            Self::Expression(expression) => {
                if expression.len() > MAX_CEL_BYTES.saturating_sub(source.len()) {
                    return Err(budget());
                }
                // Verify the leaf as an independent expression before embedding it.
                parse_cel(expression)?;
                source.push_str(expression);
                source.push('\n');
            }
            Self::Group(group) => {
                if group.conditions.is_empty()
                    || matches!(group.op, QueryBoolean::Not) && group.conditions.len() != 1
                {
                    return Err(DataError::new(
                        UiMessage::error_data_query_condition_group_arity(),
                    ));
                }
                let join = match group.op {
                    QueryBoolean::All => " && ",
                    QueryBoolean::Any => " || ",
                    QueryBoolean::Not => {
                        source.push('!');
                        ""
                    }
                };
                for (index, condition) in group.conditions.iter().enumerate() {
                    if index != 0 {
                        source.push_str(join);
                    }
                    condition.write_cel(source, depth + 1, nodes)?;
                }
            }
        }
        source.push(')');
        if source.len() > MAX_CEL_BYTES {
            return Err(budget());
        }
        Ok(())
    }
}
