use crate::error::{
    DisplayError, DisplayResult, Error, FormatCode::EnvPair as EnvPairCode, Result,
};
use crate::{impl_de_by_from_str, impl_ser_by_to_string};
use std::str::FromStr;

#[derive(Display, Debug, Clone)]
#[display(fmt = "{}", pair)]
pub struct EnvPairWithExist {
    already_exist: bool,
    pair: EnvPair,
}
impl_de_by_from_str!(EnvPairWithExist);
impl_ser_by_to_string!(EnvPairWithExist);
impl EnvPairWithExist {
    /// 使用此函式前需確保 line 非空字串
    pub fn process_line(line: &str, env_vec: &mut Vec<Self>) {
        let env = line.split_whitespace().next().unwrap();
        if let Some(p) = env_vec.iter_mut().find(|p| env == p.pair.key) {
            if let Ok(val) = std::env::var(env) {
                p.pair.val = val.clone();
                p.already_exist = true;
            }
        } else if let Ok(val) = std::env::var(env) {
            env_vec.push(EnvPairWithExist {
                already_exist: true,
                pair: EnvPair {
                    key: env.to_owned(),
                    val,
                },
            });
        }
    }
    pub fn iter_new_env<'a, T>(it: T) -> impl Iterator<Item = (&'a str, &'a str)>
    where
        T: IntoIterator<Item = &'a Self>,
    {
        it.into_iter().filter_map(|p| {
            if p.already_exist {
                None
            } else {
                Some((p.pair.key.as_str(), p.pair.val.as_str()))
            }
        })
    }
    pub fn sort(v: &mut Vec<Self>) {
        v.sort_by(|a, b| a.pair.key.cmp(&b.pair.key));
    }
}
impl FromStr for EnvPairWithExist {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(EnvPairWithExist {
            pair: EnvPair::from_str(s)?,
            already_exist: false,
        })
    }
}

#[derive(Display, Debug, Clone, Eq, PartialEq)]
#[display(fmt = "{}={}", key, val)]
pub struct EnvPair {
    pub key: String,
    pub val: String,
}
impl_ser_by_to_string!(EnvPair);
impl_de_by_from_str!(EnvPair);
impl EnvPair {
    pub fn new(line: &str, env_vec: &[Self]) -> Option<Self> {
        let env = line.split_whitespace().next().unwrap();
        if let Ok(val) = std::env::var(env) {
            // TODO: how to resolve conflict? what if it's also in env_vec?
            Some(EnvPair {
                key: env.to_owned(),
                val,
            })
        } else {
            env_vec.iter().find(|p| env == p.key).map(|p| p.clone())
        }
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
