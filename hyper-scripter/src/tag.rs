use crate::error::{Error, FormatCode::Tag as TagCode};
use crate::util::illegal_name;
use crate::{impl_de_by_from_str, impl_ser_by_to_string};
use fxhash::FxHashSet as HashSet;
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
            if f.mandatory {
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TagFilter {
    tags: Vec<TagControl>,
    pub append: bool,
    pub mandatory: bool,
}
impl_de_by_from_str!(TagFilter);
impl_ser_by_to_string!(TagFilter);

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
        if illegal_name(s) {
            log::error!("標籤格式不符：{}", s);
            return Err(Error::Format(TagCode, s.to_owned()));
        }
        Ok(Tag(s.to_owned()))
    }
}
impl FromStr for TagControl {
    type Err = Error;
    fn from_str(mut s: &str) -> std::result::Result<Self, Error> {
        let allow = if s.starts_with('^') {
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
const MANDATORY_SUFFIX: &str = "!";
const APPEND_PREFIX: &str = "+";
impl FromStr for TagFilter {
    type Err = Error;
    fn from_str(mut s: &str) -> std::result::Result<Self, Error> {
        let append = if s.starts_with(APPEND_PREFIX) {
            s = &s[APPEND_PREFIX.len()..];
            true
        } else {
            false
        };

        let mandatory = if s.ends_with(MANDATORY_SUFFIX) {
            s = &s[0..(s.len() - MANDATORY_SUFFIX.len())];
            true
        } else {
            false
        };

        let mut tags = vec![];
        for filter in s.split(',') {
            tags.push(filter.parse()?);
        }
        if tags.is_empty() {
            return Err(Error::Format(TagCode, s.to_owned()));
        }
        Ok(TagFilter {
            tags,
            append,
            mandatory,
        })
    }
}

impl Display for TagFilter {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        let mut first = true;
        if self.append {
            write!(w, "{}", APPEND_PREFIX)?;
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
        if self.mandatory {
            write!(w, "{}", MANDATORY_SUFFIX)?;
        }
        Ok(())
    }
}
impl TagFilter {
    pub fn push(&mut self, flow: Self) {
        if flow.append {
            self.tags.extend(flow.tags.into_iter());
        } else {
            *self = flow
        }
    }
    pub fn fill_allowed_map<U>(self, set: &mut std::collections::HashSet<Tag, U>)
    where
        U: std::hash::BuildHasher,
    {
        for control in self.tags.into_iter() {
            if control.allow {
                // NOTE: `match_all` 是特殊的，不用被外界知道，雖然知道了也不會怎樣
                if control.tag.match_all() {
                    continue;
                }
                set.insert(control.tag);
            } else {
                if control.tag.match_all() {
                    set.clear(); // XXX: is this the right thing to do?
                    continue;
                }
                set.remove(&control.tag);
            }
        }
    }
    pub fn into_allowed_iter(self) -> impl Iterator<Item = Tag> {
        let mut set = HashSet::default();
        self.fill_allowed_map(&mut set);
        set.into_iter()
    }
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
