use serde::de::{self, Deserialize, Deserializer, Visitor};
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
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

impl fmt::Display for ScopeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str_rep = match self {
            ScopeValue::Wildcard => "*",
            ScopeValue::Scope(s) => s,
        };
        write!(f, "{}", str_rep)
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

#[derive(PartialEq, Clone, Debug)]
pub struct ScopeEntry {
    pub parent: ScopeValue,
    pub child: ScopeValue,
}

impl PartialOrd for ScopeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.parent, &self.child, &other.parent, &other.child) {
            (ScopeValue::Wildcard, ScopeValue::Wildcard, _, _) => Some(Ordering::Greater),
            (p, ScopeValue::Wildcard, p2, _) if p == p2 => Some(Ordering::Greater),
            (ScopeValue::Wildcard, c, _, c2) if c == c2 => Some(Ordering::Greater),
            (p, c, p2, c2) if p == p2 && c == c2 => Some(Ordering::Greater),
            (_, _, _, _) => Some(Ordering::Less),
        }
    }
}

impl fmt::Display for ScopeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.parent, self.child)
    }
}

impl TryFrom<&str> for ScopeEntry {
    type Error = &'static str;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = val.split(':').collect();

        match parts.len() {
            2 => {
                let parent = ScopeValue::from(parts[0]);
                let child = ScopeValue::from(parts[1]);
                Ok(ScopeEntry { parent, child })
            }
            _ => Err("ScopeEntry must be single level"),
        }
    }
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
        match ScopeEntry::try_from(value) {
            Ok(s) => Ok(s),
            Err(e) => Err(de::Error::custom(e)),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::ScopeEntry;
    use std::convert::TryFrom;

    fn assert_scope(outer: &str, inner: &str) {
        let outer = ScopeEntry::try_from(outer).unwrap();
        let inner = ScopeEntry::try_from(inner).unwrap();
        assert!(outer > inner);
    }

    fn assert_not_scope(outer: &str, inner: &str) {
        let outer = ScopeEntry::try_from(outer).unwrap();
        let inner = ScopeEntry::try_from(inner).unwrap();
        assert!(!(outer > inner));
    }
    #[test]
    fn wildcard_outer_allows_all() {
        assert_scope("*:*", "*:*");
        assert_scope("*:*", "*:foo");
        assert_scope("*:*", "foo:foo");
    }

    #[test]
    fn wildcard_inner_denies_most() {
        assert_not_scope("foo:*", "*:*");
        assert_not_scope("*:foo", "*:*");
        assert_not_scope("foo:foo", "*:*");
    }

    #[test]
    fn wildcard_outer_parent_works() {
        assert_scope("*:bees", "*:bees");
        assert_scope("*:bees", "foo:bees");
        assert_not_scope("*:foo", "*:bar");
        assert_not_scope("*:foo", "foo:bar");
    }

    #[test]
    fn wildcard_outer_child_works() {
        assert_scope("bees:*", "bees:*");
        assert_scope("bees:*", "bees:bees");
        assert_not_scope("bees:*", "foo:*");
        assert_not_scope("bees:*", "foo:bees");
    }

    #[test]
    fn no_wildcard() {
        assert_scope("bees:bees", "bees:bees");
        assert_not_scope("bees:bees", "foo:bees");
        assert_not_scope("bees:bees", "bees:foo");
        assert_not_scope("bees:bees", "bar:foo");
        assert_not_scope("bees:bees", "foo:foo");
    }
}
