use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Util {
    pub is_hidden: bool,
    pub name: &'static str,
    pub category: &'static str,
    pub content: &'static str,
}

mod get_all_utils {
    include!(concat!(env!("OUT_DIR"), "/get_all_utils.rs"));
}

lazy_static::lazy_static! {
    static ref HIDDEN_SET: HashSet<&'static str> = {
        let mut set = HashSet::<&str>::new();
        set.insert("util/common");
        set.insert("util/hs_path");
        set
    };
}

pub fn get_all() -> Vec<Util> {
    get_all_utils::get_all()
        .into_iter()
        .map(|(name, category, content)| Util {
            is_hidden: HIDDEN_SET.contains(name),
            name,
            category,
            content,
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_get_all() {
        let utils = get_all();
        let comm = utils
            .iter()
            .find(|u| u.name == "common.rb")
            .expect("找不到應該存在的工具");
        assert!(comm.is_hidden, "{:?} 有問題", comm);
        assert_eq!(comm.category, "rb", "{:?} 有問題", comm);
        assert_eq!(include_str!("../util/common.rb"), comm.content);

        let banish = utils
            .iter()
            .find(|u| u.name == "banish.sh")
            .expect("找不到應該存在的工具");
        assert!(!banish.is_hidden, "{:?} 有問題", banish);
        assert_eq!(banish.category, "sh", "{:?} 有問題", banish);
        assert_eq!(include_str!("../util/banish.sh"), banish.content);

        assert_eq!(
            None,
            utils.iter().find(|u| u.name == "not-exist.sh"),
            "找到了不存在的工具"
        );
    }
}
