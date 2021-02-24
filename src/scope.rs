use std::fmt;

use serde::de::{self, Deserialize, Deserializer, Visitor};

#[derive(Clone, Debug)]
pub enum ScopeValue {
    Scope(String),
    Wildcard,
}

impl From<&str> for ScopeValue {
    fn from(val: &str) -> Self {
        match val {
            "*" => Self::Wildcard,
            _ => Self::Scope(String::from(val)),
        }
    }
}

impl<'de> Deserialize<'de> for ScopeValue {
    fn deserialize<D>(deserializer: D) -> Result<ScopeValue, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ScopeValueVisitor)
    }
}

struct ScopeValueVisitor;

impl<'de> Visitor<'de> for ScopeValueVisitor {
    type Value = ScopeValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string or *")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ScopeValue::from(value))
    }
}

#[derive(Clone, Debug)]
pub struct ScopeEntry {
    parent: ScopeValue,
    child: ScopeValue,
}

impl<'de> Deserialize<'de> for ScopeEntry {
    fn deserialize<D>(deserializer: D) -> Result<ScopeEntry, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ScopeEntryVisitor)
    }
}

struct ScopeEntryVisitor;

impl<'de> Visitor<'de> for ScopeEntryVisitor {
    type Value = ScopeEntry;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid scope")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let parts: Vec<&str> = value.split(":").collect();

        match parts.len() {
            2 => {
                let parent = ScopeValue::from(parts[0]);
                let child = ScopeValue::from(parts[1]);
                Ok(ScopeEntry { parent, child })
            }
            _ => Err(de::Error::custom("ScopeEntry must be single level")),
        }
    }
}
