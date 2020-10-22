use std::fmt::{Display, Formatter, Result};

pub trait TreeValue {
    fn display(&self) -> &str;
}

pub struct TreeNode<T: TreeValue> {
    display: T,
    child: Vec<TreeNode<T>>,
}

struct TreeFmtOption {
    is_done: Vec<bool>,
}

impl<'a, T: TreeValue> TreeNode<T> {
    fn fmt_lists(f: &mut Formatter<'_>, list: &[Self], opt: &mut TreeFmtOption) -> Result {
        if list.len() == 0 {
            return Ok(());
        }

        for node in list.iter().take(list.len() - 1) {
            write!(f, "\n")?;
            node.fmt_with(f, opt, false)?;
        }
        write!(f, "\n")?;
        list.last().unwrap().fmt_with(f, opt, true)?;
        Ok(())
    }
    fn fmt_with(
        &self,
        f: &mut Formatter<'_>,
        opt: &mut TreeFmtOption,
        self_is_end: bool,
    ) -> Result {
        for is_done in opt.is_done.iter().take(opt.is_done.len() - 1) {
            if *is_done {
                write!(f, "   ")?;
            } else {
                write!(f, "│  ")?;
            }
        }
        if !self_is_end {
            write!(f, "├── ")?;
        } else {
            write!(f, "└── ")?;
            *opt.is_done.last_mut().unwrap() = true;
        }
        opt.is_done.push(self_is_end);
        // TODO: 葉子特別顯示！
        write!(f, "{}", self.display.display())?;
        Self::fmt_lists(f, &self.child, opt)?;
        opt.is_done.pop();
        Ok(())
    }
}
impl<'a, T: TreeValue> Display for TreeNode<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.display.display())?;
        Self::fmt_lists(
            f,
            &self.child,
            &mut TreeFmtOption {
                is_done: vec![false],
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    impl TreeValue for &'static str {
        fn display(&self) -> &str {
            self
        }
    }
    #[test]
    fn test_display_tree() {
        let t = TreeNode {
            display: "aaa",
            child: vec![
                TreeNode {
                    display: "bbb",
                    child: vec![
                        TreeNode {
                            display: "ccc",
                            child: vec![
                                TreeNode {
                                    display: "ddd",
                                    child: vec![],
                                },
                                TreeNode {
                                    display: "eee",
                                    child: vec![],
                                },
                            ],
                        },
                        TreeNode {
                            display: "fff",
                            child: vec![],
                        },
                    ],
                },
                TreeNode {
                    display: "xxx",
                    child: vec![TreeNode {
                        display: "yyy",
                        child: vec![],
                    }],
                },
            ],
        };
        let ans = "
aaa
├── bbb
│  ├── ccc
│  │  ├── ddd
│  │  └── eee
│  └── fff
└── xxx
   └── yyy"
            .trim();
        assert_eq!(t.to_string(), ans);
    }
}
