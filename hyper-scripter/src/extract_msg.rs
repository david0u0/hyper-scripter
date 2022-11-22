const HELP_KEY: &str = "[HS_HELP]:";
const ENV_KEY: &str = "[HS_ENV]:";
const ENV_HELP_KEY: &str = "[HS_ENV_HELP]:";

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
    extract_env_from_content_help_aware(content).map(|(_, s)| s)
}

/// 第一個布林值為真代表 HS_ENV，為假代表 HS_ENV_HELP
pub fn extract_env_from_content_help_aware(content: &str) -> impl Iterator<Item = (bool, &str)> {
    // TODO: avoid traversing twice
    let env_iter = extract_msg_from_content(content, ENV_KEY).map(|s| (true, s));
    let env_help_iter = extract_msg_from_content(content, ENV_HELP_KEY).map(|s| (false, s));
    env_iter.chain(env_help_iter).filter_map(|(b, s)| {
        let s = s.trim();
        if s.is_empty() {
            None
        } else {
            Some((b, s))
        }
    })
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
    use super::*;
    fn extract_help(content: &str, long: bool) -> Vec<&str> {
        let iter = extract_help_from_content(content);
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
        let short = extract_help(content, false);
        let long = extract_help(content, true);
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
        let short2 = extract_help(&appended, false);
        let long2 = extract_help(&appended, true);
        assert_eq!(short, short2);
        assert_eq!(long, long2);
    }
    #[test]
    fn test_extract_empty() {
        let content = "
        // 不要解析我
        fn this_is_a_test() -> bool {}

        // [HS_HOLP]:我是最後一行，還拼錯字…";
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
        [HS_ENV_HELP]: env_help2
        [HS_ENV_HELP]: env_help3
        [HS_ENV]: env3
        [HS_ENV_HELP]: env_help4
        ";
        let mut v: Vec<_> = extract_env_from_content_help_aware(content).collect();
        v.sort();
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
