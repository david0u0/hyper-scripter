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
    /// ```
    /// use hyper_scripter::env_pair::EnvPair;
    /// use std::collections::HashMap;
    /// type ENV = HashMap<&'static str, &'static str>;
    ///
    /// fn map_env_pairs(v: &[EnvPair]) -> Vec<(&str, &str)> {
    ///     v.iter().map(|e| (e.key.as_str(), e.val.as_str())).collect()
    /// }
    ///
    /// let mut mock_env: ENV = Default::default();
    /// fn get(key: &str, mock_env: &ENV) -> Option<String> {
    ///     mock_env.get(key).map(|s| s.to_string())
    /// }
    ///
    /// let mut env_vec = vec![];
    /// EnvPair::process_line("VAR1 blah blah", &mut env_vec, |k| get(k, &mock_env));
    /// assert_eq!(env_vec, &[]);
    ///
    /// mock_env.insert("VAR1", "first value");
    /// EnvPair::process_line("VAR1 blah blah", &mut env_vec, |k| get(k, &mock_env));
    /// assert_eq!(map_env_pairs(&env_vec), &[("VAR1", "first value")]);
    ///
    /// mock_env.insert("VAR1", "second value");
    /// EnvPair::process_line("VAR1 blah blah", &mut env_vec, |k| get(k, &mock_env));
    /// assert_eq!(map_env_pairs(&env_vec), &[("VAR1", "first value")], "Existing value should be stronger");
    ///
    /// mock_env.insert("VAR2", "second value");
    /// EnvPair::process_line("VAR2 blah blah", &mut env_vec, |k| get(k, &mock_env));
    /// assert_eq!(map_env_pairs(&env_vec), &[("VAR1", "first value"), ("VAR2", "second value")]);
    /// ```
    pub fn process_line(
        line: &str,
        env_vec: &mut Vec<Self>,
        env_getter: impl FnOnce(&str) -> Option<String>,
    ) {
        let env = line.split_whitespace().next().unwrap();
        if env_vec.iter().find(|p| env == p.key).is_some() {
            // previous env is stronger, use it
        } else if let Some(val) = env_getter(env) {
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
