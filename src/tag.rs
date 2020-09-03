use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TagFilters(Vec<TagFilter>);
impl<'de> Deserialize<'de> for TagFilters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        let filters = TagFilters::from_str(s).unwrap();
        Ok(filters)
    }
}
impl Serialize for TagFilters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TagFilter {
    allow: bool,
    tag: Tag,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(String);
impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl Tag {
    pub fn match_all(&self) -> bool {
        // TODO: loop invariant 優化
        &self.0 == "all"
    }
}
impl FromStr for Tag {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Error> {
        // TODO: 檢查格式
        if s.len() == 0 {
            Err(Error::Format(s.to_owned()))
        } else {
            Ok(Tag(s.to_owned()))
        }
    }
}
impl FromStr for TagFilter {
    type Err = Error;
    fn from_str(tag: &str) -> std::result::Result<Self, Error> {
        let mut s = tag;
        let allow = if s.starts_with("-") {
            s = &s[1..s.len()];
            false
        } else if s.starts_with("+") {
            s = &s[1..s.len()];
            true
        } else {
            true
        };
        Ok(TagFilter {
            tag: Tag::from_str(s)?,
            allow,
        })
    }
}
impl FromStr for TagFilters {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Error> {
        let mut inner = vec![];
        for filter in s.split(",") {
            if filter.len() > 0 {
                inner.push(TagFilter::from_str(filter)?);
            }
        }
        Ok(TagFilters(inner))
    }
}

impl std::fmt::Display for TagFilters {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for f in &self.0 {
            if !first {
                write!(w, ",")?;
            }
            first = false;
            if !f.allow {
                write!(w, "-")?;
            }
            write!(w, "{}", f.tag.0)?;
        }
        Ok(())
    }
}
impl TagFilters {
    pub fn merge(&mut self, mut other: Self) {
        self.0.append(&mut other.0);
    }
    pub fn into_allowed_iter(self) -> impl Iterator<Item = Tag> {
        self.0.into_iter().filter_map(|f| {
            // NOTE: `match_all` 是特殊的，不用被外界知道，雖然知道了也不會怎樣
            if f.allow && !f.tag.match_all() {
                Some(f.tag)
            } else {
                None
            }
        })
    }
    pub fn filter(&self, tags: &[Tag]) -> bool {
        let mut pass = false;
        for filter in self.0.iter() {
            // TODO: 優化
            if filter.tag.match_all() || tags.contains(&filter.tag) {
                pass = filter.allow;
            }
        }
        pass
    }
}
