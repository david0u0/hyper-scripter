// TODO: change all `String` to `Cow`?
use std::fmt::{Display, Formatter, Result as FmtResult};
use unicode_width::UnicodeWidthStr;

mod word_wrap;

const PADDING: usize = 2;

#[derive(Debug)]
pub struct Cell {
    content: String,
    len: usize,
}

impl Cell {
    pub fn new(content: String) -> Cell {
        Cell {
            len: content.width(),
            content,
        }
    }
    pub fn new_with_len(content: String, len: usize) -> Cell {
        Cell { content, len }
    }
}

#[derive(Debug)]
pub struct Collumn {
    name: &'static str,
    fixed: bool,
    max_len: usize,

    is_fixed_this_time: bool,
    start_pos_this_time: usize,
}

impl Collumn {
    pub const fn new(name: &'static str) -> Self {
        Collumn {
            name,
            max_len: name.len(),
            fixed: false,
            is_fixed_this_time: false,
            start_pos_this_time: 0,
        }
    }
    pub const fn new_fixed(name: &'static str) -> Self {
        Collumn {
            name,
            max_len: name.len(),
            fixed: true,
            is_fixed_this_time: true,
            start_pos_this_time: 0,
        }
    }
}

#[derive(Debug)]
pub struct Table {
    rows: Vec<Vec<Cell>>,
    cols: Vec<Collumn>,
    width: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct DisplayTable<'a> {
    table: &'a Table,
    avg: usize,
}

impl Table {
    pub fn new(cols: Vec<Collumn>) -> Self {
        Table {
            rows: vec![],
            cols,
            width: 0,
        }
    }
    pub fn add_row(&mut self, row: Vec<Cell>) {
        fn set_max(og: &mut usize, new: usize) {
            if *og < new {
                *og = new;
            }
        }
        for (i, cell) in row.iter().enumerate() {
            set_max(&mut self.cols[i].max_len, cell.len);
        }
        self.rows.push(row);
    }
    pub fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    pub fn display(&mut self) -> DisplayTable<'_> {
        let (remaining, total_dyn) = self.compute_init_remaining();
        let mut avg = 0;
        if remaining > 0 {
            avg = self.compute_fix_this_time(remaining, total_dyn);
        }
        self.compute_start_pos(avg);
        DisplayTable { table: self, avg }
    }

    /// (remaining, total dynamic cell)
    fn compute_init_remaining(&mut self) -> (usize, usize) {
        let mut remaining = self.width as i32;
        let mut total_non_fixed = 0;
        // fixed col
        for col in self.cols.iter_mut() {
            if col.fixed {
                remaining -= col.max_len as i32;
            } else {
                total_non_fixed += 1;
                col.is_fixed_this_time = false;
            }
        }
        // padding
        remaining -= ((self.cols.len() - 1) * PADDING) as i32;

        if remaining <= 0 {
            (0, total_non_fixed)
        } else {
            (remaining as usize, total_non_fixed)
        }
    }
    fn compute_fix_this_time(&mut self, mut remaining: usize, mut total_dyn: usize) -> usize {
        fn compute_fix_once<'a>(
            cols: &'a mut [Collumn],
            remaining: &mut usize,
            total_dyn: &mut usize,
            avg: &mut usize,
        ) -> (&'a mut [Collumn], bool) {
            let mut first_non_fixed = None;
            let mut changed = false;
            for (i, col) in cols.iter_mut().enumerate() {
                if col.is_fixed_this_time {
                    continue;
                } else if *avg > col.max_len {
                    *total_dyn -= 1;
                    *remaining -= col.max_len;
                    col.is_fixed_this_time = true;
                    changed = true;
                    if *total_dyn == 0 {
                        break;
                    }
                    *avg = *remaining / *total_dyn;
                } else {
                    if first_non_fixed.is_none() {
                        first_non_fixed = Some(i);
                    }
                }
            }
            if !changed {
                (cols, true)
            } else if let Some(first_non_fixed) = first_non_fixed {
                (&mut cols[first_non_fixed..], false)
            } else {
                // assert total_dyn == 0
                (cols, true)
            }
        }

        if total_dyn == 0 {
            return 0;
        }
        let mut cols = &mut self.cols[..];
        let mut avg = remaining / total_dyn;
        loop {
            let (new_cols, ok) = compute_fix_once(cols, &mut remaining, &mut total_dyn, &mut avg);
            if ok {
                break;
            }
            cols = new_cols;
        }
        avg
    }
    fn compute_start_pos(&mut self, avg: usize) {
        let mut cur_pos = 0;
        for col in self.cols.iter_mut() {
            col.start_pos_this_time = cur_pos;
            cur_pos += if col.is_fixed_this_time || avg == 0 {
                // avg == 0 代表怎樣都塞不下，放棄了
                col.max_len
            } else {
                avg
            };
            cur_pos += PADDING;
        }
    }
}

#[derive(Default)]
struct MultiLineManager<'a> {
    content: Vec<Vec<(usize, &'a str)>>,
}
impl<'a> MultiLineManager<'a> {
    fn process(&mut self, idx: usize, mut s: &'a str, max_len: usize) {
        let mut cur_line_num = 0;
        loop {
            if s.is_empty() {
                break;
            }
            if cur_line_num == self.content.len() {
                self.content.push(vec![]);
            }
            let cur_line = &mut self.content[cur_line_num];
            cur_line_num += 1;

            let (s1, s2) = word_wrap::split(s, max_len);
            cur_line.push((idx, s1));
            s = s2;
        }
    }
}

impl<'a> Display for DisplayTable<'a> {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        let DisplayTable { table, avg } = *self;
        let give_up = avg == 0;
        let mut cur_pos = 0;
        for col in table.cols.iter() {
            let front_spaces = col.start_pos_this_time - cur_pos;
            for _ in 0..front_spaces {
                write!(w, " ")?;
            }
            let len = col.name.width();
            let (content, len) = if !give_up && !col.is_fixed_this_time && avg < len {
                // TODO: more lines!
                (&col.name[0..avg], avg)
            } else {
                (&col.name[..], len)
            };

            write!(w, "{}", content)?;
            cur_pos = col.start_pos_this_time + len;
        }
        for row in table.rows.iter() {
            writeln!(w, "")?;
            let mut cur_pos = 0;
            let mut multi_lines = MultiLineManager::default();
            for (i, cell) in row.iter().enumerate() {
                let col = &table.cols[i];
                let front_spaces = col.start_pos_this_time - cur_pos;
                for _ in 0..front_spaces {
                    write!(w, " ")?;
                }

                let (content, len) = if !give_up && !col.is_fixed_this_time && avg < cell.len {
                    // add more lines!
                    let (s1, s2) = word_wrap::split(&cell.content, avg);
                    multi_lines.process(i, s2, avg);
                    (s1, s1.width())
                } else {
                    (&cell.content[..], cell.len)
                };

                write!(w, "{}", content)?;
                cur_pos = col.start_pos_this_time + len;
            }
            for line in multi_lines.content.into_iter() {
                writeln!(w, "")?;
                let mut cur_pos = 0;
                for (i, content) in line.into_iter() {
                    let col = &table.cols[i];
                    let front_spaces = col.start_pos_this_time - cur_pos;
                    for _ in 0..front_spaces {
                        write!(w, " ")?;
                    }
                    write!(w, "{}", content)?;
                    cur_pos = col.start_pos_this_time + content.width(); // 不再考慮顯示寬度的問題…
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn c(s: &str) -> Cell {
        Cell::new(s.to_owned())
    }
    #[test]
    fn test_table() {
        fn one_test(width: u16) -> usize {
            const NAME: &str = "a/very/long/name/so/long/it/may/not/fit";
            let mut t = Table::new(vec![
                Collumn::new_fixed("name"),
                Collumn::new("type"),
                Collumn::new_fixed("write"),
                Collumn::new("execute"),
                Collumn::new("help msg"),
            ]);
            t.set_width(width);
            for _ in 0..5 {
                t.add_row(vec![
                    c(NAME),
                    c("docker"),
                    c("12-08 2022"),
                    c("12-08 2022(9999999)"),
                    c("a super long 測試msg, so long this 測試訊息 can't even fit in, what a terrible測試的tragdy..."),
                ]);
                t.add_row(vec![
                    c(NAME),
                    c("another"),
                    c("12-08 2022"),
                    c("12-08 2022"),
                    c("a super long msg, so long it can't even fit in, but luckily there's no unicode..."),
                ]);
                t.add_row(vec![
                    c(NAME),
                    c("docker"),
                    c("12-08 2022"),
                    c("12-08 2022(9999999)"),
                    c("測試測試測試測試測試測試測試測試測試測試測試測試測試測試測試測試"),
                ]);
            }
            let s = t.display().to_string();
            assert_eq!(s.matches(NAME).count(), 15, "name shouldn't break...");
            println!("{}", s);
            s.lines().count()
        }

        for i in 1..10 {
            one_test(i * i * 4);
        }

        assert_eq!(one_test(0), 16);
        assert_eq!(one_test(50), 16, "screen too small, should fall back");
        assert_ne!(one_test(100), 16, "should be multi lines!");
        assert_eq!(one_test(400), 16);
    }
}
