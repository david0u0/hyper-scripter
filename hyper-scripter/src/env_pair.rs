use crate::error::{DisplayError, DisplayResult, FormatCode::EnvPair as EnvPairCode};
use crate::util::{impl_de_by_from_str, impl_ser_by_to_string};
use std::str::FromStr;

#[derive(Display, Debug, Clone, Eq, PartialEq)]
#[display(fmt = "{}={}", key, val)]
pub struct EnvPair {
    pub key: String,
    pub val: String,
}
impl_ser_by_to_string!(EnvPair);
impl_de_by_from_str!(EnvPair);

impl EnvPair {
    /// 使用此函式前需確保 line 非空字串
    pub fn process_line(line: &str, env_vec: &mut Vec<Self>) {
        let env = line.split_whitespace().next().unwrap();
        if env_vec.iter().find(|p| env == p.key).is_some() {
            // previous env is stronger, use it
        } else if let Ok(val) = std::env::var(env) {
            env_vec.push(EnvPair {
                key: env.to_owned(),
                val,
            });
        }
    }
    pub fn sort(v: &mut Vec<Self>) {
        v.sort_by(|a, b| a.key.cmp(&b.key));
    }
}
impl FromStr for EnvPair {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        if let Some((key, val)) = s.split_once('=') {
            Ok(EnvPair {
                key: key.to_owned(),
                val: val.to_owned(),
            })
        } else {
            EnvPairCode.to_display_res(s.to_owned())
        }
    }
}
