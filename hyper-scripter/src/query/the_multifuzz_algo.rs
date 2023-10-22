use crate::fuzzy::{is_prefix, FuzzKey};
use crate::SEP;
use std::borrow::Cow;
use std::cmp::Ordering;

pub(super) trait MultiFuzzObj: FuzzKey {
    fn beats(&self, other: &Self) -> bool;
}

struct MultiFuzzTuple<T: MultiFuzzObj> {
    obj: T,
    is_ans: bool,
    visited: bool,
}
impl<T: MultiFuzzObj> MultiFuzzTuple<T> {
    fn new(obj: T) -> Self {
        MultiFuzzTuple {
            obj,
            is_ans: false,
            visited: false,
        }
    }
    fn new_ans(obj: T) -> Self {
        MultiFuzzTuple {
            obj,
            is_ans: true,
            visited: false,
        }
    }
    fn fuzz_key(&self) -> Cow<'_, str> {
        self.obj.fuzz_key()
    }
    fn cmp(&self, other: &Self) -> Ordering {
        other.fuzz_key().len().cmp(&self.fuzz_key().len())
    }
}

fn find_dag_sink<T: MultiFuzzObj>(
    offset: usize,
    candidates: &mut [MultiFuzzTuple<T>],
    best_sink: &mut Option<usize>,
) {
    candidates[offset].visited = true;
    let cur_key = candidates[offset].fuzz_key();
    // SAFETY: 同一個元素的鍵不可能被拜訪第二次，而且唯一的可變性發生在 `visited` 欄位
    let cur_key = unsafe { &*(cur_key.as_ref() as *const str) };
    let mut prefix_found = false;
    for i in offset + 1..candidates.len() {
        if candidates[i].visited {
            continue;
        }
        let other_key = candidates[i].fuzz_key();
        if other_key.len() != cur_key.len() && is_prefix(&other_key, cur_key, SEP) {
            prefix_found = true;
            find_dag_sink(i, candidates, best_sink);
        }
    }

    if !prefix_found {
        // 沉沒點！
        if let Some(best_sink) = best_sink {
            if candidates[offset].obj.beats(&candidates[*best_sink].obj) {
                *best_sink = offset;
            }
        } else {
            *best_sink = Some(offset)
        }
    }
}

/// with several elements equally maximum, the **FIRST** position will be returned
fn find_max_and_ans_pos<T: MultiFuzzObj>(candidates: &[MultiFuzzTuple<T>]) -> (usize, usize) {
    let mut max_pos = 0;
    let mut ans_pos = 0;
    for i in 1..candidates.len() {
        if candidates[i].is_ans {
            ans_pos = i;
        }
        if candidates[i].obj.beats(&candidates[max_pos].obj) {
            max_pos = i;
        }
    }
    (max_pos, ans_pos)
}

/// 從多個模糊搜分數相近者中裁決出「最合適者」。函式過程不太直覺，故在此詳述。
/// 參數：正解(ans)即分數最高者，其它(others)即其它分數相近的候選人。
///       應保證正解的長度不大於所有其它人
///
/// 1. 建立一個有向無環圖，路徑由長節點指向短節點
/// 2. 從「最強者」（根據 MultiFuzzObj::beats）出發，找到所有沉沒點
/// 3. 於所有沉沒點中選出最強者
/// 4. 若最強者為正解之前綴（重排序），回傳正解
pub(super) fn the_multifuzz_algo<T: MultiFuzzObj>(ans: T, others: Vec<T>) -> T {
    let mut candidates: Vec<_> = others.into_iter().map(|t| MultiFuzzTuple::new(t)).collect();
    candidates.push(MultiFuzzTuple::new_ans(ans));
    // 由長至短排序
    candidates.sort_by(MultiFuzzTuple::cmp);
    let (max_pos, ans_pos) = find_max_and_ans_pos(&candidates);
    let mut ret_pos = None;
    find_dag_sink(max_pos, &mut candidates, &mut ret_pos);
    let mut ret_pos = ret_pos.unwrap();
    if is_prefix(
        &candidates[ret_pos].fuzz_key(),
        &candidates[ans_pos].fuzz_key(),
        SEP,
    ) {
        ret_pos = ans_pos;
    }
    candidates.into_iter().skip(ret_pos).next().unwrap().obj
}

#[cfg(test)]
mod test {
    use super::*;
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    struct MyObj {
        s: &'static str,
        order: usize,
    }
    impl MyObj {
        fn new(s: &'static str) -> Self {
            MyObj { s, order: 0 }
        }
    }
    impl FuzzKey for MyObj {
        fn fuzz_key(&self) -> Cow<'_, str> {
            std::borrow::Cow::Borrowed(self.s)
        }
    }
    impl MultiFuzzObj for MyObj {
        fn beats(&self, other: &Self) -> bool {
            other.order > self.order
        }
    }
    fn reorder<const S: usize>(mut arr: [&mut MyObj; S]) {
        for (i, obj) in arr.iter_mut().enumerate() {
            obj.order = i;
        }
    }

    #[test]
    fn test_the_multifuzz_algo() {
        let mut ans = MyObj::new("dir");
        let mut other_p = MyObj::new("dir/a");
        let mut other = MyObj::new("dother");
        macro_rules! run_the_algo {
            () => {
                the_multifuzz_algo(ans, vec![other, other_p])
            };
        }

        reorder([&mut other_p, &mut other, &mut ans]);
        assert_eq!(ans, run_the_algo!());
        reorder([&mut other, &mut other_p, &mut ans]);
        assert_eq!(other, run_the_algo!());
        reorder([&mut ans, &mut other, &mut other_p]);
        assert_eq!(ans, run_the_algo!());

        assert_eq!(ans, the_multifuzz_algo(ans, vec![]));

        assert_eq!(ans, the_multifuzz_algo(ans, vec![other_p]));
        assert_eq!(ans, the_multifuzz_algo(ans, vec![other]));
        reorder([&mut other, &mut other_p, &mut ans]);
        assert_eq!(ans, the_multifuzz_algo(ans, vec![other_p]));
        assert_eq!(other, the_multifuzz_algo(ans, vec![other]));
    }
    #[test]
    fn test_the_multi_sink_multifuzz_algo() {
        let mut root = MyObj::new("a/b/c/d");
        let mut b1_1 = MyObj::new("a/b/c");
        let mut b1_2 = MyObj::new("b/c");
        let mut b2_1 = MyObj::new("a/c/d");
        macro_rules! run_the_algo {
            () => {
                the_multifuzz_algo(b1_2, vec![root, b1_1, b2_1])
            };
        }

        reorder([&mut root, &mut b1_1, &mut b2_1, &mut b1_2]);
        assert_eq!(b2_1, run_the_algo!());
        reorder([&mut root, &mut b1_1, &mut b1_2, &mut b2_1]);
        assert_eq!(b1_2, run_the_algo!());

        reorder([&mut b1_1, &mut root, &mut b1_2, &mut b2_1]);
        assert_eq!(b1_2, run_the_algo!());
        reorder([&mut b1_1, &mut root, &mut b2_1, &mut b1_2]);
        assert_eq!(b1_2, run_the_algo!());
    }
    #[test]
    fn test_multifuzz_determined_ans() {
        let mut abcd = MyObj::new("a/b/c/d");
        let mut bacd = MyObj::new("b/a/c/d");
        let mut cabd = MyObj::new("c/a/b/d");
        let mut dacb = MyObj::new("d/a/c/b");
        let mut acbd = MyObj::new("a/c/b/d");

        let mut abc = MyObj::new("a/b/c");
        let mut cab = MyObj::new("c/a/b");

        reorder([&mut abcd, &mut bacd, &mut cabd, &mut dacb, &mut acbd]);
        assert_eq!(abcd, the_multifuzz_algo(abcd, vec![bacd, cabd, dacb, acbd]));
        assert_eq!(bacd, the_multifuzz_algo(bacd, vec![abcd, cabd, dacb, acbd]));
        assert_eq!(dacb, the_multifuzz_algo(dacb, vec![abcd, bacd, cabd, acbd]));

        // prefix is still preferred
        reorder([&mut abc, &mut cab]);
        assert_eq!(
            abc,
            the_multifuzz_algo(abc, vec![cab, abcd, bacd, cabd, dacb])
        );
        assert_eq!(
            cab,
            the_multifuzz_algo(cab, vec![abc, abcd, bacd, cabd, dacb])
        );
    }
}
