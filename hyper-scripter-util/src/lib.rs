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
        set.insert("util/hs_env");
        set
    };
}

pub fn get_all() -> Vec<Util> {
    get_all_utils::get_all()
        .into_iter()
        .map(|u| Util {
            is_hidden: u.is_hidden,
            name: u.name,
            category: u.category,
            content: u.content,
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
            .find(|u| u.name == "util/common")
            .expect("找不到應該存在的工具");
        assert!(comm.is_hidden, "{:?} 有問題", comm);
        assert_eq!(comm.category, "rb", "{:?} 有問題", comm);
        assert_eq!(include_str!("../util/common.rb"), comm.content);

        let import = utils
            .iter()
            .find(|u| u.name == "util/import")
            .expect("找不到應該存在的工具");
        assert!(!import.is_hidden, "{:?} 有問題", import);
        assert_eq!(import.category, "rb", "{:?} 有問題", import);
        assert_eq!(include_str!("../util/import.rb"), import.content);

        assert_eq!(
            None,
            utils.iter().find(|u| u.name == "not-exist"),
            "找到了不存在的工具"
        );
    }
}
