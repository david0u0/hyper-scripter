use crate::error::{DisplayError, DisplayResult, FormatCode::Tag as TagCode};
use crate::script_type::ScriptType;
use crate::util::illegal_name;
use crate::util::{impl_de_by_from_str, impl_ser_by_to_string};
use fxhash::FxHashSet as HashSet;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

pub type TagSet = HashSet<Tag>;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TagSelectorGroup(Vec<TagSelector>);
impl TagSelectorGroup {
    pub fn push(&mut self, selector: TagSelector) {
        if selector.append {
            self.0.push(selector);
        } else {
            self.0 = vec![selector];
        }
    }
    pub fn select(&self, tags: &TagSet, ty: &ScriptType) -> bool {
        let mut pass = false;
        for f in self.0.iter() {
            let res = f.select(tags, ty);
            match res {
                SelectResult::MandatoryFalse => return false,
                SelectResult::Normal(res) => pass = res,
                SelectResult::None => (),
            }
        }
        pass
    }
}
impl From<TagSelector> for TagSelectorGroup {
    fn from(t: TagSelector) -> Self {
        TagSelectorGroup(vec![t])
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TagSelector {
    tags: TagGroup,
    pub append: bool,
}
impl_de_by_from_str!(TagSelector);
impl_ser_by_to_string!(TagSelector);

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TagGroup(Vec<TagControl>);
impl_de_by_from_str!(TagGroup);
impl_ser_by_to_string!(TagGroup);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TagControl {
    allow: bool,
    mandatory: bool,
    tag: TagOrType,
}

#[derive(Debug, Clone, Eq, PartialEq, Display)]
pub enum TagOrType {
    #[display(fmt = "@{}", _0)]
    Type(ScriptType),
    #[display(fmt = "{}", _0)]
    Tag(Tag),
}
impl_de_by_from_str!(TagOrType);
impl_ser_by_to_string!(TagOrType);
impl FromStr for TagOrType {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        Ok(if s.starts_with('@') {
            TagOrType::Type(s[1..].parse()?)
        } else {
            TagOrType::Tag(s.parse()?)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
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
    pub fn new_unchecked(s: String) -> Self {
        Tag(s)
    }
}
impl FromStr for Tag {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        if illegal_name(s) {
            log::error!("標籤格式不符：{}", s);
            return TagCode.to_display_res(s.to_owned());
        }
        Ok(Tag(s.to_owned()))
    }
}
impl FromStr for TagControl {
    type Err = DisplayError;
    fn from_str(mut s: &str) -> DisplayResult<Self> {
        let allow = if s.starts_with('^') {
            s = &s[1..s.len()];
            false
        } else {
            true
        };
        let mandatory = if s.ends_with(MANDATORY_SUFFIX) {
            s = &s[0..(s.len() - MANDATORY_SUFFIX.len())];
            true
        } else {
            false
        };
        Ok(TagControl {
            tag: s.parse()?,
            allow,
            mandatory,
        })
    }
}
const MANDATORY_SUFFIX: &str = "!";
const APPEND_PREFIX: &str = "+";
impl FromStr for TagSelector {
    type Err = DisplayError;
    fn from_str(mut s: &str) -> DisplayResult<Self> {
        let append = if s.starts_with(APPEND_PREFIX) {
            s = &s[APPEND_PREFIX.len()..];
            true
        } else {
            false
        };

        Ok(TagSelector {
            tags: s.parse()?,
            append,
        })
    }
}

impl Display for TagSelector {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        if self.append {
            write!(w, "{}", APPEND_PREFIX)?;
        }
        write!(w, "{}", self.tags)?;
        Ok(())
    }
}

impl TagSelector {
    pub fn push(&mut self, other: Self) {
        if other.append {
            self.tags.0.extend(other.tags.0.into_iter());
        } else {
            *self = other
        }
    }
    pub fn fill_allowed_map<U>(self, set: &mut std::collections::HashSet<Tag, U>)
    where
        U: std::hash::BuildHasher,
    {
        for control in self.tags.0.into_iter() {
            let tag = match control.tag {
                TagOrType::Type(_) => continue, // 類型篩選，跳過
                TagOrType::Tag(t) => t,
            };
            if control.allow {
                // NOTE: `match_all` 是特殊的，不用被外界知道，雖然知道了也不會怎樣
                if tag.match_all() {
                    continue;
                }
                set.insert(tag);
            } else {
                if tag.match_all() {
                    set.clear(); // XXX: is this the right thing to do?
                    continue;
                }
                set.remove(&tag);
            }
        }
    }
    pub fn into_allowed_iter(self) -> impl Iterator<Item = Tag> {
        let mut set = HashSet::default();
        self.fill_allowed_map(&mut set);
        set.into_iter()
    }
    pub fn select(&self, tags: &TagSet, ty: &ScriptType) -> SelectResult {
        self.tags.select(tags, ty)
    }
}

impl FromStr for TagGroup {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        let mut tags = vec![];
        if !s.is_empty() {
            for ctrl in s.split(',') {
                tags.push(ctrl.parse()?);
            }
        }
        Ok(TagGroup(tags))
    }
}

impl Display for TagGroup {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        let mut first = true;
        for f in self.0.iter() {
            if !first {
                write!(w, ",")?;
            }
            first = false;
            if !f.allow {
                write!(w, "^")?;
            }
            write!(w, "{}", f.tag)?;
            if f.mandatory {
                write!(w, "{}", MANDATORY_SUFFIX)?;
            }
        }
        Ok(())
    }
}

pub enum SelectResult {
    None,
    MandatoryFalse,
    Normal(bool),
}
impl SelectResult {
    pub fn is_true(&self) -> bool {
        matches!(self, SelectResult::Normal(true))
    }
}

impl TagGroup {
    pub fn select(&self, tags: &TagSet, ty: &ScriptType) -> SelectResult {
        let mut pass = SelectResult::None;
        for ctrl in self.0.iter() {
            let hit = match &ctrl.tag {
                TagOrType::Type(t) => ty == t,
                TagOrType::Tag(t) => t.match_all() || tags.contains(t),
            };
            if ctrl.mandatory {
                if ctrl.allow {
                    if !hit {
                        return SelectResult::MandatoryFalse;
                    }
                } else {
                    if hit {
                        return SelectResult::MandatoryFalse;
                    }
                }
            }
            if hit {
                pass = SelectResult::Normal(ctrl.allow);
            }
        }
        pass
    }
}
