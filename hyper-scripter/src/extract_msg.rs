const HELP_KEY: &str = "[HS_HELP]:";
const ENV_KEY: &str = "[HS_ENV]:";

pub fn extract_env_from_content(content: &str) -> impl ExactSizeIterator<Item = &str> {
    extract_msg_from_content(content, ENV_KEY, true)
        .into_iter()
        .map(str::trim)
}
pub fn extract_help_from_content(content: &str, long: bool) -> impl ExactSizeIterator<Item = &str> {
    fn trim_first_white(s: &str) -> &str {
        if let Some(s) = s.strip_prefix(' ') {
            s
        } else {
            s
        }
    }
    extract_msg_from_content(content, HELP_KEY, long)
        .into_iter()
        .map(|s| trim_first_white(s))
}

fn extract_msg_from_content<'a>(mut content: &'a str, key: &str, long: bool) -> Vec<&'a str> {
    let mut ans = vec![];
    if let Some(pos) = content.find(key) {
        content = &content[pos..];
        let new_line_pos = content.find('\n').unwrap_or_else(|| content.len());
        ans.push(&content[key.len()..new_line_pos]);
        if !long {
            return ans;
        }
        content = &content[new_line_pos..];
    } else {
        return ans;
    }

    while let Some(pos) = content.find(key) {
        content = &content[pos..];
        let new_line_pos = content.find('\n').unwrap_or_else(|| content.len());
        ans.push(&content[key.len()..new_line_pos]);

        content = &content[new_line_pos..];
    }

    ans
}

#[cfg(test)]
mod test {
    use super::*;
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
