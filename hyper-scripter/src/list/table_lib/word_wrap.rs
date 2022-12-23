const SPACE: char = ' ';

pub fn split(s: &str, max_len: usize) -> (&str, &str) {
    if s.len() <= max_len {
        // 不再考慮顯示寬度的問題…
        return (s, "");
    }

    let mut s1 = &s[..max_len];
    let mut next_start = max_len;
    if s1.chars().last().unwrap() != SPACE {
        if let Some(space_pos) = s1.rfind(SPACE) {
            next_start = space_pos + 1;
            s1 = &s1[..space_pos + 1];
        }
    }

    (s1, &s[next_start..])
}
