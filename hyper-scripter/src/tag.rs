use crate::error::{Error, FormatCode::Tag as TagCode};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TagFilterGroup(Vec<TagFilter>);
impl TagFilterGroup {
    pub fn push(&mut self, filter: TagFilter) {
        if filter.append {
            self.0.push(filter);
        } else {
            self.0 = vec![filter];
        }
    }
    pub fn filter(&self, tags: &[&Tag]) -> bool {
        let mut pass = false;
        for f in self.0.iter() {
            let res = f.filter(tags);
            if f.obligation {
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

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TagFilter {
    tags: Vec<TagControl>,
    pub append: bool,
    pub obligation: bool,
}
impl<'de> Deserialize<'de> for TagFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        // TODO: unwrap?
        Ok(s.parse().unwrap())
    }
}
impl Serialize for TagFilter {
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}
impl FromStr for Tag {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        let re = Regex::new(&format!(r"^(\w|\d|-|_)+$")).unwrap();
        if !re.is_match(s) {
            log::error!("標籤格式不符：{}", s);
            return Err(Error::Format(TagCode, s.to_owned()));
        }
        Ok(Tag(s.to_owned()))
    }
}
impl FromStr for TagControl {
    type Err = Error;
    fn from_str(mut s: &str) -> std::result::Result<Self, Error> {
        let allow = if s.starts_with("^") {
            s = &s[1..s.len()];
            false
        } else {
            true
        };
        Ok(TagControl {
            tag: s.parse()?,
            allow,
        })
    }
}
const OBLIGATION_PREFIX: &'static str = "o/";
impl FromStr for TagFilter {
    type Err = Error;
    fn from_str(mut s: &str) -> std::result::Result<Self, Error> {
        let append = if s.starts_with("+") {
            s = &s[1..];
            true
        } else {
            false
        };

        let obligation = if s.starts_with(OBLIGATION_PREFIX) {
            s = &s[OBLIGATION_PREFIX.len()..];
            true
        } else {
            false
        };

        let mut tags = vec![];
        for filter in s.split(",") {
            tags.push(filter.parse()?);
        }
        if tags.len() == 0 {
            return Err(Error::Format(TagCode, s.to_owned()));
        }
        Ok(TagFilter {
            tags,
            append,
            obligation,
        })
    }
}

impl Display for TagFilter {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        let mut first = true;
        if self.append {
            write!(w, "+")?;
        }
        if self.obligation {
            write!(w, "o/")?;
        }
        for f in self.tags.iter() {
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
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
    pub fn push(&mut self, flow: Self) {
        if flow.append {
            self.tags.extend(flow.tags.into_iter());
        } else {
            *self = flow
        }
    }
    pub fn into_allowed_iter(self) -> impl Iterator<Item = Tag> {
        self.tags.into_iter().filter_map(|f| {
            // NOTE: `match_all` 是特殊的，不用被外界知道，雖然知道了也不會怎樣
            if f.allow && !f.tag.match_all() {
                Some(f.tag)
            } else {
                None
            }
        })
    }
}
impl TagFilter {
    pub fn filter(&self, tags: &[&Tag]) -> Option<bool> {
        let mut pass: Option<bool> = None;
        for filter in self.tags.iter() {
            // TODO: 優化
            if filter.tag.match_all() || tags.contains(&&filter.tag) {
                pass = Some(filter.allow);
            }
        }
        pass
    }
}
