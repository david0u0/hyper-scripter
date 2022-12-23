use unicode_width::UnicodeWidthStr;

const SPACE: char = ' ';

pub fn split(s: &str, max_len: usize) -> (&str, &str) {
    if s.width() <= max_len {
        // 不再考慮顯示寬度的問題…
        return (s, "");
    }

    let max_len = find_char_boundary(s, max_len);
    let mut s1 = &s[..max_len];
    let mut next_start = max_len;
    if s1.chars().last().unwrap() != SPACE {
        if s.as_bytes()[next_start] == SPACE as u8 {
            next_start += 1;
        } else if let Some(space_pos) = s1.rfind(SPACE) {
            next_start = space_pos + 1;
            s1 = &s1[..space_pos + 1];
        }
    }

    (s1, &s[next_start..])
}

pub fn find_char_boundary(s: &str, max_len: usize) -> usize {
    let mut bound = 0;
    let mut width = 0;
    for i in 1..s.len() {
        if !s.is_char_boundary(i) {
            continue;
        }

        width += s[bound..i].width();
        if width <= max_len {
            bound = i;
        } else {
            break;
        }
    }
    bound
}
