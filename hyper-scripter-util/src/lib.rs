mod get_all_utils {
    include!(concat!(env!("OUT_DIR"), "/get_all_utils.rs"));
}

pub use get_all_utils::{get_all, Util};

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_get_all() {
        let utils: Vec<_> = get_all().into_iter().map(|u| *u).collect();
        let comm = utils
            .iter()
            .find(|u| u.name == "util/common")
            .expect("找不到應該存在的工具");
        assert!(comm.is_hidden, "{:?} 有問題", comm);
        assert_eq!(comm.ty, "rb", "{:?} 有問題", comm);
        assert_eq!(include_str!("../util/common.rb"), comm.content);

        let import = utils
            .iter()
            .find(|u| u.name == "util/import")
            .expect("找不到應該存在的工具");
        assert!(!import.is_hidden, "{:?} 有問題", import);
        assert_eq!(import.ty, "rb", "{:?} 有問題", import);
        assert_eq!(include_str!("../util/import.rb"), import.content);

        assert_eq!(
            None,
            utils.iter().find(|u| u.name == "not-exist"),
            "找到了不存在的工具"
        );
    }
}
