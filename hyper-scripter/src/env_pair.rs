use crate::error::{Error, Result};
use crate::{impl_de_by_from_str, impl_ser_by_to_string};
use std::str::FromStr;

#[derive(Display, Debug, Eq, PartialEq)]
#[display(fmt = "{} {}", key, val)]
pub struct EnvPair {
    pub key: String,
    pub val: String,
}
impl_ser_by_to_string!(EnvPair);
impl_de_by_from_str!(EnvPair);
impl EnvPair {
    /// 使用此函式前需確保 line 非空字串
    pub fn new(line: &str) -> Option<Self> {
        let env = line.split_whitespace().next().unwrap();
        if let Ok(val) = std::env::var(env) {
            Some(EnvPair {
                key: env.to_owned(),
                val,
            })
        } else {
            None
        }
    }
    pub fn sort(v: &mut Vec<Self>) {
        v.sort_by(|a, b| a.key.cmp(&b.key));
    }
}
impl FromStr for EnvPair {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        if let Some((key, val)) = s.split_once(' ') {
            Ok(EnvPair {
                key: key.to_owned(),
                val: val.to_owned(),
            })
        } else {
            Ok(EnvPair {
                key: s.to_owned(),
                val: String::new(),
            })
        }
    }
}
