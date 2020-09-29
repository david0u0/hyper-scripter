use crate::error::{Error, Result};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::borrow::Cow;

const MIN_SCORE: i64 = 400; // TODO: 好好決定這個魔法數字

lazy_static::lazy_static! {
    static ref MATCHER: SkimMatcherV2 = SkimMatcherV2::default();
}

pub trait FuzzKey {
    fn fuzz_key<'a>(&'a self) -> Cow<'a, str>;
}
pub fn fuzz_mut<'a, T: FuzzKey + 'a>(
    name: &str,
    iter: impl Iterator<Item = T>,
) -> Result<Option<T>> {
    let mut ans = (0, Vec::<T>::new());
    for data in iter {
        let key_tmp = data.fuzz_key();
        let key = key_tmp.as_ref();
        let score = my_fuzz(&key, name);
        // let score = MATCHER.fuzzy_match(&key, name);
        if let Some(mut score) = score {
            let len = key.chars().count();
            log::trace!("將分數正交化：{} / {}", score * 100, len);
            score = score * 100 / len as i64;
            if score > MIN_SCORE {
                if score > ans.0 {
                    ans = (score, vec![data]);
                } else if score == ans.0 {
                    ans.1.push(data);
                }
            }
        }
    }
    if ans.1.len() == 0 {
        log::warn!("模糊搜沒搜到東西");
        Ok(None)
    } else if ans.1.len() == 1 {
        log::debug!(
            "模糊搜到一個東西 {:?}",
            ans.1.iter().map(|k| k.fuzz_key()).collect::<Vec<_>>()
        );
        let first = ans.1.into_iter().next().unwrap();
        Ok(Some(first))
    } else {
        log::debug!("模糊搜到太多東西");
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
    use crate::script::AsScriptName;
    #[test]
    fn test_fuzz() {
        let _ = env_logger::try_init();
        let t1 = "測試腳本1".as_script_name().unwrap();
        let t2 = "測試腳本2".as_script_name().unwrap();
        let t3 = ".42".as_script_name().unwrap();
        let vec = vec![t1.clone(), t2, t3.clone()];

        let res = fuzz_mut("測試1", vec.clone().into_iter()).unwrap();
        assert_eq!(res, Some(t1));

        let res = fuzz_mut("42", vec.clone().into_iter()).unwrap();
        assert_eq!(res, Some(t3));

        let res = fuzz_mut("找不到", vec.clone().into_iter()).unwrap();
        assert_eq!(res, None);

        let err = fuzz_mut("測試", vec.clone().into_iter()).unwrap_err();
        let mut v = match err {
            Error::MultiFuzz(v) => v,
            _ => unreachable!(),
        };
        v.sort();
        assert_eq!(v, vec!["測試腳本1".to_owned(), "測試腳本2".to_owned()]);
    }
    #[test]
    fn test_fuzz_with_len() {
        let _ = env_logger::try_init();
        let t1 = "測試腳本1".as_script_name().unwrap();
        let t2 = "測試腳本234".as_script_name().unwrap();
        let vec = vec![t1.clone(), t2];
        let res = fuzz_mut("測試", vec.clone().into_iter()).unwrap();
        assert_eq!(res, Some(t1), "模糊搜尋無法找出較短者");
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
