use crate::error::{Error, Result};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::borrow::Cow;

const MIN_SCORE: i64 = 200; // TODO: 好好決定這個魔法數字

pub trait FuzzKey {
    fn fuzz_key<'a>(&'a self) -> Cow<'a, str>;
}
pub fn fuzz_mut<'a, T: FuzzKey + 'a>(
    name: &str,
    mut iter: impl Iterator<Item = &'a mut T>,
) -> Result<Option<&'a mut T>> {
    let matcher = SkimMatcherV2::default();
    let mut ans = (0, Vec::<&mut T>::new());
    for data in iter {
        let key_tmp = data.fuzz_key();
        let key = key_tmp.as_ref();
        let score = matcher.fuzzy_match(&key, name);
        log::trace!(
            "模糊搜尋，輸入：{}，候選人：{}，分數：{:?}",
            name,
            key,
            score
        );
        if let Some(mut score) = score {
            log::trace!("將分數正交化：{} / {}", score * 100, key.len());
            score = score * 100 / key.len() as i64;
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
        Ok(None)
    } else if ans.1.len() == 1 {
        // SAFETY: 不會再用到這個向量了，而且 &mut 也沒有 Drop 特徵，安啦
        let first = unsafe { std::ptr::read(&ans.1[0]) };
        Ok(Some(first))
    } else {
        Err(Error::MultiFuzz(
            ans.1
                .into_iter()
                .map(|data| data.fuzz_key().as_ref().to_owned())
                .collect(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::script::AsScriptName;
    #[test]
    fn test_fuzz() {
        let _ = env_logger::try_init();
        let mut t1 = "測試腳本1".as_script_name().unwrap();
        let t2 = "測試腳本2".as_script_name().unwrap();
        let mut t3 = ".42".as_script_name().unwrap();
        let mut vec = vec![t1.clone(), t2, t3.clone()];

        let res = fuzz_mut("測試1", vec.iter_mut()).unwrap();
        assert_eq!(res, Some(&mut t1));

        let res = fuzz_mut("42", vec.iter_mut()).unwrap();
        assert_eq!(res, Some(&mut t3));

        let res = fuzz_mut("找不到", vec.iter_mut()).unwrap();
        assert_eq!(res, None);

        let err = fuzz_mut("測試", vec.iter_mut()).unwrap_err();
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
        let mut t1 = "測試腳本1".as_script_name().unwrap();
        let t2 = "測試腳本234".as_script_name().unwrap();
        let mut vec = vec![t1.clone(), t2];
        let res = fuzz_mut("測試", vec.iter_mut()).unwrap();
        assert_eq!(res, Some(&mut t1), "模糊搜尋無法找出較短者");
    }
}
