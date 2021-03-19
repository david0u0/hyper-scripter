use crate::error::{Error, Result};
use futures::future::join_all;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::borrow::Cow;
use tokio::task::spawn_blocking;

const MID_SCORE: i64 = 1000; // TODO: 好好決定這個魔法數字

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FuzzSimilarity {
    High,
    Low,
}
pub use FuzzSimilarity::*;
impl FuzzSimilarity {
    fn from_score(score: i64) -> Self {
        match score {
            0..=MID_SCORE => FuzzSimilarity::Low,
            _ => FuzzSimilarity::High,
        }
    }
}

lazy_static::lazy_static! {
    static ref MATCHER: SkimMatcherV2 = SkimMatcherV2::default();
}

pub trait FuzzKey {
    fn fuzz_key(&self) -> Cow<'_, str>;
}
#[derive(Copy, Clone)]
struct MyRaw(*const str);
unsafe impl Send for MyRaw {}
impl MyRaw {
    fn new(s: &str) -> Self {
        MyRaw(s.as_ref() as *const str)
    }
    unsafe fn get(&self) -> &'static str {
        &*self.0
    }
}

pub async fn fuzz<'a, T: FuzzKey + Send + 'a>(
    name: &str,
    iter: impl Iterator<Item = T>,
) -> Result<Option<(T, FuzzSimilarity)>> {
    let mut ans = (0, Vec::<T>::new());
    let raw_name = MyRaw::new(name);
    let data_vec: Vec<_> = iter.collect();
    // NOTE: 當鍵是 Cow::Owned 可能會太早釋放，一定要先存起來
    let keys: Vec<_> = data_vec.iter().map(|data| data.fuzz_key()).collect();
    let score_fut: Vec<_> = keys
        .iter()
        .map(|key| {
            let key = MyRaw::new(key.as_ref());
            spawn_blocking(move || {
                // SAFTY: 等等就會 join，故這個函式 await 完之前都不可能釋放這些字串
                let key = unsafe { key.get() };
                let score = my_fuzz(key, unsafe { raw_name.get() });

                if let Some(mut score) = score {
                    let len = key.chars().count();
                    log::trace!("將分數正交化：{} / {}", score * 100, len);
                    score = score * 100 / len as i64;
                    Some(score)
                } else {
                    None
                }
            })
        })
        .collect();

    let scores = join_all(score_fut).await;

    for (data, score) in data_vec.into_iter().zip(scores.into_iter()) {
        if let Some(score) = score? {
            if score > ans.0 {
                ans = (score, vec![data]);
            } else if score == ans.0 {
                ans.1.push(data);
            }
        }
    }
    if ans.1.len() == 0 {
        log::info!("模糊搜沒搜到東西：{}", name);
        Ok(None)
    } else if ans.1.len() == 1 {
        let first = ans.1.into_iter().next().unwrap();
        log::info!("模糊搜到一個東西 {:?}", first.fuzz_key());
        let similarity = FuzzSimilarity::from_score(ans.0);
        Ok(Some((first, similarity)))
    } else {
        log::warn!("模糊搜到太多東西");
        Err(Error::MultiFuzz(
            ans.1
                .into_iter()
                .map(|data| data.fuzz_key().as_ref().to_owned())
                .collect(),
        ))
    }
}

fn my_fuzz(choice: &str, pattern: &str) -> Option<i64> {
    let mut ans_opt = None;
    foreach_reorder(choice, "/", &mut |choice_reordered| {
        let score_opt = MATCHER.fuzzy_match(choice_reordered, pattern);
        log::trace!(
            "模糊搜尋，候選者：{}，重排列成：{}，輸入：{}，分數：{:?}",
            choice,
            choice_reordered,
            pattern,
            score_opt,
        );
        if let Some(score) = score_opt {
            if let Some(ans) = ans_opt {
                ans_opt = Some(std::cmp::max(score, ans));
            } else {
                ans_opt = score_opt;
            }
        }
    });
    ans_opt
}
fn foreach_reorder<F: FnMut(&str)>(choice: &str, sep: &str, handler: &mut F) {
    let choice_arr: Vec<_> = choice.split(sep).collect();
    let mut mem = vec![false; choice_arr.len()];
    let mut reorederd = Vec::<&str>::with_capacity(mem.len());
    recursive_reorder(&choice_arr, &mut mem, &mut reorederd, sep, handler);
}

fn recursive_reorder<'a, F: FnMut(&str)>(
    choice_arr: &[&'a str],
    mem: &mut Vec<bool>,
    reorderd: &mut Vec<&'a str>,
    sep: &str,
    handler: &mut F,
) {
    if reorderd.len() == mem.len() {
        let new_str = reorderd.join(sep);
        handler(&new_str);
    } else {
        for i in 0..mem.len() {
            if mem[i] {
                continue;
            }
            mem[i] = true;
            reorderd.push(choice_arr[i]);
            recursive_reorder(choice_arr, mem, reorderd, sep, handler);
            reorderd.pop();
            mem[i] = false;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    impl<'a> FuzzKey for &'a str {
        fn fuzz_key(&self) -> Cow<'a, str> {
            Cow::Borrowed(self)
        }
    }
    #[tokio::test(threaded_scheduler)]
    async fn test_fuzz() {
        let _ = env_logger::try_init();
        let t1 = "測試腳本1";
        let t2 = "測試腳本2";
        let t3 = ".42";
        let vec = vec![t1.clone(), t2, t3.clone()];

        let res = fuzz("測試1", vec.clone().into_iter())
            .await
            .unwrap()
            .unwrap()
            .0;
        assert_eq!(res, t1);

        let res = fuzz("42", vec.clone().into_iter())
            .await
            .unwrap()
            .unwrap()
            .0;
        assert_eq!(res, t3);

        let res = fuzz("找不到", vec.clone().into_iter()).await.unwrap();
        assert_eq!(res, None);

        let err = fuzz("測試", vec.clone().into_iter()).await.unwrap_err();
        let mut v = match err {
            Error::MultiFuzz(v) => v,
            _ => unreachable!(),
        };
        v.sort();
        assert_eq!(v, vec!["測試腳本1".to_owned(), "測試腳本2".to_owned()]);
    }
    #[tokio::test(threaded_scheduler)]
    async fn test_fuzz_with_len() {
        let _ = env_logger::try_init();
        let t1 = "測試腳本1";
        let t2 = "測試腳本234";
        let vec = vec![t1.clone(), t2];
        let res = fuzz("測試", vec.clone().into_iter())
            .await
            .unwrap()
            .unwrap()
            .0;
        assert_eq!(res, t1, "模糊搜尋無法找出較短者");
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
}
