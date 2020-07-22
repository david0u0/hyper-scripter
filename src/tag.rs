use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct TagFilters(Vec<TagFilter>);

#[derive(Debug, Clone)]
pub struct TagFilter {
    allow: bool,
    tag: Tag,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag(String);

impl FromStr for Tag {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        // TODO: 檢查格式
        if s.len() == 0 {
            Err(format!("Wrong tag format: {}", s))
        } else {
            Ok(Tag(s.to_owned()))
        }
    }
}
impl FromStr for TagFilter {
    type Err = String;
    fn from_str(tag: &str) -> std::result::Result<Self, String> {
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
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        let mut inner = vec![];
        for filter in s.split(",") {
            if filter.len() > 0 {
                inner.push(TagFilter::from_str(filter)?);
            }
        }
        Ok(TagFilters(inner))
    }
}

impl TagFilters {
    pub fn into_allowed_iter(self) -> impl Iterator<Item = Tag> {
        self.0
            .into_iter()
            .filter_map(|f| if f.allow { Some(f.tag) } else { None })
    }
    pub fn filter(&self, tags: &[Tag]) -> bool {
        let mut pass = false;
        for filter in self.0.iter() {
            // TODO: 優化
            if tags.contains(&filter.tag) {
                pass = filter.allow;
            }
        }
        pass
    }
}
