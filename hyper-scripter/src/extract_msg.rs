const HELP_KEY: &str = "[HS_HELP]:";
const ENV_KEY: &str = "[HS_ENV_HELP]:";

pub struct Iter<'a, 'b> {
    long: bool,
    done: bool,
    content: &'a str,
    key: &'b str,
}
impl<'a, 'b> Iterator for Iter<'a, 'b> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if let Some(pos) = self.content.find(self.key) {
            let content = &self.content[pos..];
            let new_line_pos = content.find('\n').unwrap_or_else(|| content.len());
            let ans = &content[self.key.len()..new_line_pos];

            self.content = &content[new_line_pos..];
            if !self.long {
                self.done = true;
            }

            Some(ans)
        } else {
            None
        }
    }
}

pub fn extract_env_from_content(content: &str) -> impl Iterator<Item = &str> {
    extract_msg_from_content(content, ENV_KEY, true).map(str::trim)
}
pub fn extract_help_from_content(content: &str, long: bool) -> impl Iterator<Item = &str> {
    fn trim_first_white(s: &str) -> &str {
        if let Some(s) = s.strip_prefix(' ') {
            s
        } else {
            s
        }
    }
    extract_msg_from_content(content, HELP_KEY, long).map(|s| trim_first_white(s))
}

fn extract_msg_from_content<'a, 'b>(content: &'a str, key: &'b str, long: bool) -> Iter<'a, 'b> {
    Iter {
        long,
        done: false,
        content,
        key,
    }
}

#[cfg(test)]
mod test {
    fn extract_help_from_content(content: &str, long: bool) -> Vec<&str> {
        super::extract_help_from_content(content, long).collect()
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
