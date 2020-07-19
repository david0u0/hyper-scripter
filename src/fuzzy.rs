use crate::script::ScriptName;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::collections::HashMap;

const MIN_SCORE: i64 = 10; // TODO: 好好決定這個魔法數字

pub fn fuzz_mut<'a, 'b, T>(
    name: &'a str,
    map: &'b mut HashMap<ScriptName, T>,
) -> Option<&'b mut T> {
    let matcher = SkimMatcherV2::default();
    let mut ans = (0, None::<&mut T>);
    for (choice, data) in map.iter_mut() {
        let score = matcher.fuzzy_match(&choice.to_string(), name);
        log::trace!(
            "模糊搜尋，輸入：{}，候選人：{:?}，分數：{:?}",
            name,
            choice,
            score
        );
        if let Some(score) = score {
            if score > ans.0 && score > MIN_SCORE {
                ans = (score, Some(data));
            }
        }
    }
    ans.1
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_fuzz() {
        let mut map = HashMap::new();
        map.insert(ScriptName::Named("測試腳本1".to_owned()), 111);
        map.insert(ScriptName::Anonymous(42), 222);

        let res = fuzz_mut("測試", &mut map);
        assert_eq!(res, Some(&mut 111));

        let res = fuzz_mut("42", &mut map);
        assert_eq!(res, Some(&mut 222));
    }
}
