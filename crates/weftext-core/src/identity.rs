use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::{Uuid, Version};

/// Persistent identity of one Weftext node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(Uuid);

impl NodeId {
    #[must_use]
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.hyphenated().fmt(formatter)
    }
}

impl FromStr for NodeId {
    type Err = NodeIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parsed = Uuid::parse_str(value).map_err(|_| NodeIdError::InvalidUuid)?;
        if parsed.get_version() != Some(Version::Random) {
            return Err(NodeIdError::NotVersionFour);
        }
        if parsed.hyphenated().to_string() != value {
            return Err(NodeIdError::NotCanonicalLowercase);
        }
        Ok(Self(parsed))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeIdError {
    InvalidUuid,
    NotVersionFour,
    NotCanonicalLowercase,
}

impl fmt::Display for NodeIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidUuid => "node ID is not a UUID",
            Self::NotVersionFour => "node ID is not UUIDv4",
            Self::NotCanonicalLowercase => "node ID is not canonical lowercase UUID text",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NodeIdError {}
