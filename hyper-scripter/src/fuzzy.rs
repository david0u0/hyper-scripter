use crate::error::Result;
use crate::state::State;
use futures::future::join_all;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::borrow::Cow;
use tokio::task::spawn_blocking;

const MID_SCORE: i64 = 4100; // TODO: 好好決定這個魔法數字

fn is_multifuzz(score: i64, best_score: i64) -> bool {
    // best_score * 0.7 < score
    best_score * 7 < score * 10
}

#[derive(Debug)]
pub enum FuzzResult<T> {
    High(T),
    Low(T),
    Multi { ans: T, others: Vec<T> },
}
pub use FuzzResult::*;
impl<T> FuzzResult<T> {
    fn new_single(ans: T, score: i64) -> Self {
        match score {
            0..=MID_SCORE => Low(ans),
            _ => High(ans),
        }
    }
    fn new_multi(ans: T, others: Vec<T>) -> Self {
        Multi { ans, others }
    }
    pub fn get_ans(self) -> T {
        match self {
            High(t) => t,
            Low(t) => t,
            Multi { ans, .. } => ans,
        }
    }
}

static MATCHER: State<SkimMatcherV2> = State::new();

pub trait FuzzKey {
    fn fuzz_key(&self) -> Cow<'_, str>;
}
impl<T: AsRef<str>> FuzzKey for T {
    fn fuzz_key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.as_ref())
    }
}

#[derive(Copy, Clone)]
struct MyRaw(*const str);
unsafe impl Send for MyRaw {}
impl MyRaw {
    fn new(s: &str) -> Self {
        MyRaw(s as *const str)
    }
    unsafe fn get(&self) -> &'static str {
        &*self.0
    }
}

pub async fn fuzz<'a, T: FuzzKey + Send + 'a>(
    name: &str,
    iter: impl Iterator<Item = T>,
    sep: &str,
) -> Result<Option<FuzzResult<T>>> {
    let raw_name = MyRaw::new(name);
    let mut data_vec: Vec<(i64, T)> = iter.map(|t| (0, t)).collect();
    // NOTE: 當鍵是 Cow::Owned 可能會太早釋放，一定要先存起來
    let keys: Vec<_> = data_vec.iter().map(|(_, data)| data.fuzz_key()).collect();
    let sep = MyRaw::new(sep);

    crate::set_once!(MATCHER, Default::default);

    let score_fut = keys.iter().map(|key| {
        let key = MyRaw::new(key.as_ref());
        spawn_blocking(move || {
            // SAFTY: 等等就會 join，故這個函式 await 完之前都不可能釋放這些字串
            let key = unsafe { key.get() };
            let score = my_fuzz(key, unsafe { raw_name.get() }, unsafe { sep.get() });

            if let Some(score) = score {
                let len = key.chars().count() as i64;
                let computed_score = score * score * 10 / len as i64;
                log::trace!(
                    "將分數正交化：{} * {} * 10 / {} = {}",
                    score,
                    score,
                    len,
                    computed_score
                );
                Some(computed_score)
            } else {
                None
            }
        })
    });

    let scores = join_all(score_fut).await;
    let mut best_score = 0;
    for (score, (score_mut, _)) in scores.into_iter().zip(data_vec.iter_mut()) {
        if let Some(score) = score? {
            best_score = std::cmp::max(best_score, score);
            *score_mut = score;
        }
    }
    if best_score == 0 {
        log::info!("模糊搜沒搜到東西 {}", name);
        return Ok(None);
    }

    let mut ans = None;
    let mut multifuzz_vec = vec![];
    for (score, data) in data_vec.into_iter() {
        if score == best_score && ans.is_none() {
            ans = Some(data);
        } else if is_multifuzz(score, best_score) {
            log::debug!("找到一個分數相近者：{} {}", data.fuzz_key(), score);
            multifuzz_vec.push(data);
        }
    }

    let ans = ans.unwrap();
    if multifuzz_vec.is_empty() {
        log::info!("模糊搜到一個東西 {:?}", ans.fuzz_key());
        Ok(Some(FuzzResult::new_single(ans, best_score)))
    } else {
        log::warn!("模糊搜到太多東西");
        Ok(Some(FuzzResult::new_multi(ans, multifuzz_vec)))
    }
}

// TODO: 把這些 sep: &str 換成標準庫的 Pattern

pub fn is_prefix(prefix: &str, target: &str, sep: &str) -> bool {
    if prefix.len() > target.len() {
        return false;
    }

    let mut found = false;
    foreach_reorder(target, sep, &mut |t| {
        foreach_reorder(prefix, sep, &mut |p| {
            if t.starts_with(p) {
                found = true;
            }
            found
        });
        found
    });

    found
}

fn my_fuzz(choice: &str, pattern: &str, sep: &str) -> Option<i64> {
    let mut ans_opt = None;
    let mut first = true;
    foreach_reorder(choice, sep, &mut |choice_reordered| {
        let score_opt = MATCHER.get().fuzzy_match(choice_reordered, pattern);
        log::trace!(
            "模糊搜尋，候選者：{}，重排列成：{}，輸入：{}，分數：{:?}",
            choice,
            choice_reordered,
            pattern,
            score_opt,
        );
        if let Some(mut score) = score_opt {
            if first {
                // NOTE: 正常排序的分數會稍微高一點
                // 例如 [a/b, b/a] 中要找 `a/b`，則前者以分毫之差勝出
                score += 1;
                log::trace!(
                    "模糊搜尋，候選者：{}，正常排序就命中，分數略提升為 {}",
                    choice,
                    score
                );
            }
            if let Some(ans) = ans_opt {
                ans_opt = Some(std::cmp::max(score, ans));
            } else {
                ans_opt = Some(score);
            }
        }
        first = false;
    });
    ans_opt
}

trait StopIndicator: Default {
    fn should_stop(&self) -> bool {
        false
    }
}
impl StopIndicator for () {}
impl StopIndicator for bool {
    fn should_stop(&self) -> bool {
        *self
    }
}
fn foreach_reorder<S: StopIndicator, F: FnMut(&str) -> S>(
    choice: &str,
    sep: &str,
    handler: &mut F,
) {
    fn recursive_reorder<'a, S: StopIndicator, F: FnMut(&str) -> S>(
        choice_arr: &[&'a str],
        mem: &mut Vec<bool>,
        reorderd: &mut Vec<&'a str>,
        sep: &str,
        handler: &mut F,
    ) -> S {
        if reorderd.len() == mem.len() {
            let new_str = reorderd.join(sep);
            handler(&new_str)
        } else {
            for i in 0..mem.len() {
                if mem[i] {
                    continue;
                }
                mem[i] = true;
                reorderd.push(choice_arr[i]);
                let indicator = recursive_reorder(choice_arr, mem, reorderd, sep, handler);
                if indicator.should_stop() {
                    return indicator;
                }
                reorderd.pop();
                mem[i] = false;
            }
            Default::default()
        }
    }

    let choice_arr: Vec<_> = choice.split(sep).collect();
    let mut mem = vec![false; choice_arr.len()];
    let mut reorederd = Vec::<&str>::with_capacity(mem.len());
    recursive_reorder(&choice_arr, &mut mem, &mut reorederd, sep, handler);
}

#[cfg(test)]
mod test {
    use super::*;

    fn extract_multifuzz<'a>(res: FuzzResult<&'a str>) -> (&'a str, Vec<&'a str>) {
        match res {
            Multi { ans, others } => {
                let mut ret = vec![];
                ret.push(ans);
                for data in others.into_iter() {
                    ret.push(data);
                }
                ret.sort();
                (ans, ret)
            }
            _ => unreachable!(),
        }
    }
    fn extract_high<'a>(res: FuzzResult<&'a str>) -> &'a str {
        match res {
            High(t) => t,
            _ => unreachable!(),
        }
    }
    async fn do_fuzz<'a>(name: &'a str, v: &'a Vec<&'a str>) -> Option<FuzzResult<&'a str>> {
        fuzz(name, v.iter().map(|s| *s), "/").await.unwrap()
    }
    #[tokio::test(threaded_scheduler)]
    async fn test_fuzz() {
        let _ = env_logger::try_init();
        let t1 = "測試腳本1";
        let t2 = "測試腳本2";
        let t3 = ".42";
        let t4 = "測腳本4試";
        let vec = vec![t1, t2, t3, t4];

        let res = do_fuzz("測試1", &vec).await.unwrap();
        assert_eq!(extract_high(res), t1);

        let res = do_fuzz("42", &vec).await.unwrap();
        assert_eq!(extract_high(res), t3);

        let res = do_fuzz("找不到", &vec).await;
        assert!(res.is_none());

        let res = do_fuzz("測試", &vec).await.unwrap();
        let (ans, v) = extract_multifuzz(res);
        assert_eq!(v, vec!["測試腳本1", "測試腳本2"]);
        assert_eq!(ans, "測試腳本1"); // 真的同分，只好以順序決定了
    }
    #[tokio::test(threaded_scheduler)]
    async fn test_fuzz_with_len() {
        let _ = env_logger::try_init();
        let t1 = "測試腳本1";
        let t2 = "測試腳本23456";
        let vec = vec![t1, t2];
        let res = do_fuzz("測試", &vec).await.unwrap();
        assert_eq!(extract_high(res), t1, "模糊搜尋無法找出較短者");
    }
    #[tokio::test(threaded_scheduler)]
    async fn test_reorder_fuzz() {
        let _ = env_logger::try_init();
        let t1 = "a/c";
        let t2 = "b/a";
        let t3 = "a/b";
        let vec = vec![t1, t2, t3];

        let res = do_fuzz("ab", &vec).await.unwrap();
        let (ans, v) = extract_multifuzz(res);
        assert_eq!(ans, "a/b");
        assert_eq!(v, vec!["a/b", "b/a"]);

        let res = do_fuzz("ba", &vec).await.unwrap();
        let (ans, v) = extract_multifuzz(res);
        assert_eq!(ans, "b/a");
        assert_eq!(v, vec!["a/b", "b/a"]);

        let res = do_fuzz("ca", &vec).await.unwrap();
        assert_eq!(extract_high(res), "a/c");

        let res = do_fuzz("a", &vec).await.unwrap();
        let (ans, v) = extract_multifuzz(res);
        assert_eq!(ans, "a/c"); // 真的同分，只好以順序決定了
        assert_eq!(v, vec!["a/b", "a/c", "b/a"]);
    }
    #[test]
    fn test_reorder() {
        let arr = "aa::bb::cc";
        let mut buffer = vec![];
        foreach_reorder(arr, "::", &mut |s| {
            buffer.push(s.to_owned());
        });
        buffer.sort();
        assert_eq!(
            vec![
                "aa::bb::cc",
                "aa::cc::bb",
                "bb::aa::cc",
                "bb::cc::aa",
                "cc::aa::bb",
                "cc::bb::aa"
            ],
            buffer
        );
    }
    #[test]
    fn test_is_prefix() {
        let sep = "::";
        assert!(is_prefix("aa", "aabb", sep));
        assert!(is_prefix("aa::bb", "bb::cc::aa", sep));
        assert!(is_prefix("c", "bb::cc::aa", sep));
        assert!(is_prefix("aa::bb", "bb::aa1", sep));

        assert!(is_prefix("aa::b", "bb::cc::aa", sep));
        assert!(is_prefix("a::bb", "bb::cc::aa", sep));

        assert!(!is_prefix("abb", "aabb", sep));
        assert!(!is_prefix("aabb", "aa::bb", sep));

        assert!(!is_prefix("aa::bb::cc", "aa::bb", sep));
        assert!(!is_prefix("aa::dd", "bb::cc::aa", sep));
    }
}
