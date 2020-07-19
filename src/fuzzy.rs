use crate::error::{Error, Result};
use crate::script::{ScriptName, ToScriptName};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::collections::HashMap;

const MIN_SCORE: i64 = 10; // TODO: 好好決定這個魔法數字

pub fn fuzz_mut<'a, 'b, T>(
    name: &'a str,
    map: &'b mut HashMap<ScriptName, T>,
    exact: bool,
) -> Result<Option<&'b mut T>> {
    if exact {
        let name = name.to_owned().to_script_name()?;
        return Ok(map.get_mut(&name));
    }
    let matcher = SkimMatcherV2::default();
    let mut ans = (0, Vec::<(&ScriptName, &mut T)>::new());
    for (choice, data) in map.iter_mut() {
        let score = matcher.fuzzy_match(&choice.to_string(), name);
        log::trace!(
            "模糊搜尋，輸入：{}，候選人：{:?}，分數：{:?}",
            name,
            choice,
            score
        );
        if let Some(score) = score {
            if score > MIN_SCORE {
                if score > ans.0 {
                    ans = (score, vec![(choice, data)]);
                } else if score == ans.0 {
                    ans.1.push((choice, data));
                }
            }
        }
    }
    if ans.1.len() == 0 {
        Ok(None)
    } else if ans.1.len() == 1 {
        // SAFETY: 不會再用到這個向量了啦
        let first = unsafe { std::ptr::read(&ans.1[0].1) };
        Ok(Some(first))
    } else {
        Err(Error::MultiFuzz(
            ans.1.into_iter().map(|(name, _)| name.clone()).collect(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_fuzz() {
        let mut map = HashMap::new();
        map.insert(ScriptName::Named("測試腳本1".to_owned()), 111);
        map.insert(ScriptName::Named("測試腳本2".to_owned()), 111);
        map.insert(ScriptName::Anonymous(42), 222);

        let res = fuzz_mut("測試1", &mut map, true).unwrap();
        assert_eq!(res, Some(&mut 111));

        let res = fuzz_mut("42", &mut map, true).unwrap();
        assert_eq!(res, Some(&mut 222));

        let res = fuzz_mut("找不到", &mut map, true).unwrap();
        assert_eq!(res, None);

        let err = fuzz_mut("測試", &mut map, true).unwrap_err();
        let mut v = if let Error::MultiFuzz(v) = err {
            v
        } else {
            unreachable!()
        };
        v.sort();
        assert_eq!(
            v,
            vec![
                ScriptName::Named("測試腳本1".to_owned()),
                ScriptName::Named("測試腳本2".to_owned()),
            ]
        );
    }
}
