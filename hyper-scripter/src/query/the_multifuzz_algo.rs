use crate::fuzzy::{is_prefix, FuzzKey};
use crate::SEP;
use std::cmp::Ordering;

pub(super) trait MultiFuzzObj: FuzzKey {
    fn cmp(&self, other: &Self) -> Ordering;
}

/// 從多個模糊搜分數相近者中裁決出「最合適者」。函式過程不太直覺，故在此詳述。
/// 參數：正解(ans)即分數最高者，其它(others)即其它分數相近的候選人。
///
/// 1. 從所有候選人中依照 MultiFuzzObj::cmp 選出「最大者」，稱贏家(winner)。
///     - 包含正解本身
/// 2. 檢查正解是否為贏家之前綴
///     (i) 否 => 回傳贏家
///     (ii) 是 => 回傳正解
pub(super) fn the_multifuzz_algo<T: MultiFuzzObj>(ans: T, others: Vec<T>) -> T {
    let winner = others.into_iter().max_by(T::cmp);

    if let Some(winner) = winner {
        if matches!(ans.cmp(&winner), Ordering::Greater) {
            ans
        } else {
            if is_prefix(&ans.fuzz_key(), &winner.fuzz_key(), SEP) {
                ans
            } else {
                winner
            }
        }
    } else {
        ans
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::borrow::Cow;
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    struct MyObj {
        s: &'static str,
        order: u8,
    }
    impl FuzzKey for MyObj {
        fn fuzz_key(&self) -> Cow<'_, str> {
            std::borrow::Cow::Borrowed(self.s)
        }
    }
    impl MultiFuzzObj for MyObj {
        fn cmp(&self, other: &Self) -> Ordering {
            other.order.cmp(&self.order)
        }
    }

    #[test]
    fn test_the_multifuzz_algo() {
        let mut ans = MyObj {
            s: "dir",
            order: 99,
        };
        let mut other = vec![
            MyObj {
                s: "dir/a",
                order: 2,
            },
            MyObj {
                s: "dother",
                order: 3,
            },
        ];

        assert_eq!(ans, the_multifuzz_algo(ans, other.clone()));
        other[0].order = 4;
        assert_eq!(other[1], the_multifuzz_algo(ans, other.clone()));
        ans.order = 0;
        assert_eq!(ans, the_multifuzz_algo(ans, other.clone()));

        assert_eq!(ans, the_multifuzz_algo(ans, vec![]));
        assert_eq!(ans, the_multifuzz_algo(ans, vec![other[0]]));
        assert_eq!(ans, the_multifuzz_algo(ans, vec![other[1]]));
        ans.order = 99;
        assert_eq!(ans, the_multifuzz_algo(ans, vec![other[0]]));
        assert_eq!(other[1], the_multifuzz_algo(ans, vec![other[1]]));
    }
}
