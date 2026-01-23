use crate::util::impl_ser_and_display_by_as_ref;

const HELP_KEY: &str = "[HS_HELP]:";
const ENV_KEY: &str = "[HS_ENV]:";
const ENV_HELP_KEY: &str = "[HS_ENV_HELP]:";

const KEYS: &[&str] = &[HELP_KEY, ENV_KEY, ENV_HELP_KEY];

pub struct Message {
    start: usize,
    s: String,
}
impl AsRef<str> for Message {
    fn as_ref(&self) -> &str {
        let s = &self.s[self.start..];
        if let Some(s) = s.strip_prefix(' ') {
            s
        } else {
            s
        }
    }
}
impl_ser_and_display_by_as_ref!(Message);

pub struct Iter<'a, I> {
    content_iter: I,
    keys: &'a [&'a str],
}
impl<'a, I: Iterator<Item = String>> Iterator for Iter<'a, I> {
    type Item = (usize, Message);
    fn next(&mut self) -> Option<Self::Item> {
        let Some(content) = self.content_iter.next() else {
            return None;
        };

        if content.trim().is_empty() {
            return self.next();
        }

        for (i, key) in self.keys.iter().enumerate() {
            if let Some(pos) = content.find(key) {
                return Some((
                    i,
                    Message {
                        start: pos + key.len(),
                        s: content,
                    },
                ));
            }
        }
        None
    }
}

pub fn extract_all_help_from_content(
    content: impl Iterator<Item = String>,
) -> impl Iterator<Item = (usize, Message)> {
    extract_msg_from_content(content, KEYS)
}

/// 第一個布林值為真代表 HS_ENV，為假代表 HS_ENV_HELP
pub fn extract_env_from_content_help_aware(
    content: impl Iterator<Item = String>,
) -> impl Iterator<Item = (bool, Message)> {
    let env_iter = extract_msg_from_content(content, KEYS);
    env_iter.filter_map(|(i, s)| match i {
        1 => Some((true, s)),
        2 => Some((false, s)),
        _ => None,
    })
}

pub fn extract_help_from_content<'a>(
    content: impl Iterator<Item = String>,
) -> impl Iterator<Item = Message> {
    extract_msg_from_content(content, KEYS).filter_map(|(i, s)| if i == 0 { Some(s) } else { None })
}

fn extract_msg_from_content<'a, I>(content_iter: I, keys: &'a [&'a str]) -> Iter<'a, I>
where
    I: Iterator<Item = String>,
{
    Iter { keys, content_iter }
}

#[cfg(test)]
mod test {
    use super::*;
    fn extract_help(content: &str, long: bool) -> Vec<String> {
        let iter = extract_help_from_content(content.lines().map(str::to_string));
        let iter = iter.map(|x| x.to_string());
        if long {
            iter.collect()
        } else {
            iter.take(1).collect()
        }
    }

    #[test]
    fn test_extract_help() {
        let content = "

        // [HS_HELP]:   解析我吧  

        // [HS_HELP]:
        // [HS_HELP]: 第二行
        // [HS_NOHELP]: 不要解析我QQ
        // [HS_HELP]: 我沒救了";
        let short = extract_help(content, false);
        let long = extract_help(content, true);
        assert_eq!(short, vec!["  解析我吧  "]);
        assert_eq!(long, vec!["  解析我吧  ", "", "第二行",]);
    }
    #[test]
    fn test_extract_empty() {
        let content = "
        // 不要解析我
        fn this_is_a_test() -> bool {}

        // [HS_HELP]:我是最後一行";
        let short = extract_help(content, false);
        let long = extract_help(content, true);
        assert_eq!(short.len(), 0);
        assert_eq!(long.len(), 0);
    }
    #[test]
    fn test_extract_env() {
        let content = "
        [HS_ENV_HELP]: env_help1
        [HS_ENV]: env1

        [HS_ENV]: env2
        [HS_HELP]: this is a help
        [HS_ENV_HELP]: env_help2

        [HS_ENV_HELP]: env_help3
        [HS_HELP]: this is a help
        [HS_ENV]: env3
        [HS_ENV_HELP]: env_help4

        掰
        [HS_ENV]: this is useless
        ";
        let mut v: Vec<_> =
            extract_env_from_content_help_aware(content.lines().map(str::to_string))
                .map(|(x, y)| (x, y.to_string()))
                .collect();
        v.sort();
        let v: Vec<_> = v.iter().map(|(x, y)| (*x, y.as_str())).collect();
        assert_eq!(
            v,
            vec![
                (false, "env_help1"),
                (false, "env_help2"),
                (false, "env_help3"),
                (false, "env_help4"),
                (true, "env1"),
                (true, "env2"),
                (true, "env3"),
            ]
        );
    }
}
