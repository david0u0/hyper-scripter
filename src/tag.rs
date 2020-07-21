use std::str::FromStr;

#[derive(Debug)]
pub struct TagFilters(Vec<TagFilter>);

#[derive(Debug)]
pub struct TagFilter {
    is_black_list: bool,
    tag_name: String,
}
#[derive(Debug)]
pub enum TagControl {
    Literal(String),
    Local,
    Parents,
}

impl FromStr for TagFilter {
    type Err = String;
    fn from_str(mut s: &str) -> std::result::Result<Self, String> {
        // TODO: 檢查白名單黑名單
        let is_black_list;
        if s.starts_with("-") {
            is_black_list = true;
            s = &s[1..s.len()];
        } else if s.starts_with("+") {
            is_black_list = false;
            s = &s[1..s.len()];
        } else {
            is_black_list = false;
        }
        let control = match s {
            "%L" => TagControl::Local,
            "%P" => TagControl::Parents,
            _ => {
                // TODO: 檢查合法性
                TagControl::Literal(s.to_owned())
            }
        };
        Ok(TagFilter {
            is_black_list,
            tag_name: s.to_owned(),
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
