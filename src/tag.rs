use crate::error::{Error, FormatCode::Tag as TagCode};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TagFilterGroup(Vec<TagFilter>);
impl TagFilterGroup {
    pub fn push(&mut self, filter: TagFilter) {
        self.0.push(filter);
    }
    pub fn filter(&self, tags: &[Tag]) -> bool {
        let mut pass = false;
        for f in self.0.iter() {
            let res = f.filter(tags);
            if f.must {
                if res != Some(true) {
                    return false;
                }
            } else if let Some(res) = res {
                pass = res;
            }
        }
        pass
    }
}
impl From<TagFilter> for TagFilterGroup {
    fn from(t: TagFilter) -> Self {
        TagFilterGroup(vec![t])
    }
}
fn is_false(t: &bool) -> bool {
    !t
}
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TagFilter {
    pub filter: TagControlFlow,
    #[serde(default, skip_serializing_if = "is_false")]
    pub must: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TagControlFlow(Vec<TagControl>);
impl<'de> Deserialize<'de> for TagControlFlow {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        // TODO: unwrap?
        Ok(FromStr::from_str(s).unwrap())
    }
}
impl Serialize for TagControlFlow {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TagControl {
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
            Err(Error::Format(TagCode, s.to_owned()))
        } else {
            Ok(Tag(s.to_owned()))
        }
    }
}
impl FromStr for TagControl {
    type Err = Error;
    fn from_str(tag: &str) -> std::result::Result<Self, Error> {
        let mut s = tag;
        let allow = if s.starts_with("^") {
            s = &s[1..s.len()];
            false
        } else if s.starts_with("+") {
            s = &s[1..s.len()];
            true
        } else {
            true
        };
        Ok(TagControl {
            tag: Tag::from_str(s)?,
            allow,
        })
    }
}
impl FromStr for TagFilter {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Error> {
        Ok(TagFilter {
            filter: FromStr::from_str(s)?,
            must: false,
        })
    }
}
impl FromStr for TagControlFlow {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Error> {
        let mut tags = vec![];
        for filter in s.split(",") {
            if filter.len() > 0 {
                tags.push(TagControl::from_str(filter)?);
            }
        }
        Ok(TagControlFlow(tags))
    }
}

impl std::fmt::Display for TagControlFlow {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for f in self.0.iter() {
            if !first {
                write!(w, ",")?;
            }
            first = false;
            if !f.allow {
                write!(w, "^")?;
            }
            write!(w, "{}", f.tag.0)?;
        }
        Ok(())
    }
}
impl TagFilter {
    pub fn into_allowed_iter(self) -> impl Iterator<Item = Tag> {
        self.filter.0.into_iter().filter_map(|f| {
            // NOTE: `match_all` 是特殊的，不用被外界知道，雖然知道了也不會怎樣
            if f.allow && !f.tag.match_all() {
                Some(f.tag)
            } else {
                None
            }
        })
    }
    pub fn filter(&self, tags: &[Tag]) -> Option<bool> {
        let mut pass: Option<bool> = None;
        for filter in self.filter.0.iter() {
            // TODO: 優化
            if filter.tag.match_all() || tags.contains(&filter.tag) {
                pass = Some(filter.allow);
            }
        }
        pass
    }
}
