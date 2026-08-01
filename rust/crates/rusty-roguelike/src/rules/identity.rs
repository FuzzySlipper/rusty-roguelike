use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

pub const MAX_ROGUELIKE_ID_BYTES: usize = 64;
pub const ROGUELIKE_ID_PATTERN: &str = "^[a-z0-9._-]+$";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
pub struct RoguelikeId(String);

impl RoguelikeId {
    pub fn parse(value: impl Into<String>) -> Result<Self, RoguelikeIdentityError> {
        let value = value.into();
        if value.is_empty() {
            return Err(RoguelikeIdentityError::Empty);
        }
        if value.len() > MAX_ROGUELIKE_ID_BYTES {
            return Err(RoguelikeIdentityError::TooLong {
                actual: value.len(),
                maximum: MAX_ROGUELIKE_ID_BYTES,
            });
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        }) {
            return Err(RoguelikeIdentityError::InvalidCharacter { value });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RoguelikeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for RoguelikeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RoguelikeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoguelikeIdentityError {
    Empty,
    TooLong { actual: usize, maximum: usize },
    InvalidCharacter { value: String },
}

impl fmt::Display for RoguelikeIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Roguelike identity: {self:?}")
    }
}

impl std::error::Error for RoguelikeIdentityError {}
