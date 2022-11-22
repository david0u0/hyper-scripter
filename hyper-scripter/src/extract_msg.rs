use crate::error::{Error, Result};
use crate::{impl_de_by_from_str, impl_ser_by_to_string};
use std::str::FromStr;

const HELP_KEY: &str = "[HS_HELP]:";
const ENV_KEY: &str = "[HS_ENV_HELP]:";

pub struct Iter<'a, 'b> {
    content: &'a str,
    key: &'b str,
}
impl<'a, 'b> Iterator for Iter<'a, 'b> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pos) = self.content.find(self.key) {
            let content = &self.content[pos..];
            let new_line_pos = content.find('\n').unwrap_or_else(|| content.len());
            let ans = &content[self.key.len()..new_line_pos];

            self.content = &content[new_line_pos..];
            Some(ans)
        } else {
            None
        }
    }
}

pub fn extract_env_from_content(content: &str) -> impl Iterator<Item = &str> {
    extract_msg_from_content(content, ENV_KEY).filter_map(|s| {
        let s = s.trim();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    })
}

#[derive(Display)]
#[display(fmt = "{} {}", key, val)]
pub struct EnvPair {
    pub key: String,
    pub val: String,
}
impl_ser_by_to_string!(EnvPair);
impl_de_by_from_str!(EnvPair);
impl EnvPair {
    /// 使用此函式前需確保 lines 中沒有空字串
    pub fn collect_envs<'a, T: AsRef<str>>(lines: &'a [T]) -> Vec<Self> {
        let mut v = vec![];
        for line in lines.iter() {
            let env = line.as_ref().split_whitespace().next().unwrap();
            if let Ok(val) = std::env::var(env) {
                v.push(EnvPair {
                    key: env.to_owned(),
                    val,
                });
            }
        }
        v.sort_by(|a, b| a.key.cmp(&b.key));
        v
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
            Err(Error::msg("env format"))
        }
    }
}

pub fn extract_help_from_content(content: &str) -> impl Iterator<Item = &str> {
    fn trim_first_white(s: &str) -> &str {
        if let Some(s) = s.strip_prefix(' ') {
            s
        } else {
            s
        }
    }
    extract_msg_from_content(content, HELP_KEY).map(|s| trim_first_white(s))
}

fn extract_msg_from_content<'a, 'b>(content: &'a str, key: &'b str) -> Iter<'a, 'b> {
    Iter { content, key }
}

#[cfg(test)]
mod test {
    fn extract_help_from_content(content: &str, long: bool) -> Vec<&str> {
        let iter = super::extract_help_from_content(content);
        if long {
            iter.collect()
        } else {
            iter.take(1).collect()
        }
    }
    #[test]
    fn test_extract_help() {
        let content = "
        // 不要解析我
        // [HS_HELP]:   解析我吧  

        // [HS_NOHELP]: 不要解析我QQ
        // [HS_HELP]:
        fn this_is_a_test() -> bool {}

        # [HS_HELP]: 解析我吧，雖然我是個失敗的註解

        //  前面有些 垃圾[HS_HELP]:我是最後一行";
        let short = extract_help_from_content(content, false);
        let long = extract_help_from_content(content, true);
        assert_eq!(short, vec!["  解析我吧  "]);
        assert_eq!(
            long,
            vec![
                "  解析我吧  ",
                "",
                "解析我吧，雖然我是個失敗的註解",
                "我是最後一行"
            ]
        );

        let appended = format!("{}\n 真．最後一行", content);
        let short2 = extract_help_from_content(&appended, false);
        let long2 = extract_help_from_content(&appended, true);
        assert_eq!(short, short2);
        assert_eq!(long, long2);
    }
    #[test]
    fn test_extract_empty() {
        let content = "
        // 不要解析我
        fn this_is_a_test() -> bool {}

        // [HS_HOLP]:我是最後一行，還拼錯字…";
        let short = extract_help_from_content(content, false);
        let long = extract_help_from_content(content, true);
        assert_eq!(short.len(), 0);
        assert_eq!(long.len(), 0);
    }
}
