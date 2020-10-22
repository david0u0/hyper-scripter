use std::cmp::Ordering;
use std::io::{Error, Write};

type Result = std::result::Result<(), Error>;

pub trait TreeValue: Ord + Eq {}

#[derive(Debug, PartialEq, Eq)]
pub struct NonLeaf<'a, T: TreeValue> {
    value: &'a str,
    childs: Vec<TreeNode<'a, T>>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum TreeNode<'a, T: TreeValue> {
    Leaf(T),
    NonLeaf(NonLeaf<'a, T>),
}
impl<'a, T: TreeValue> PartialOrd for TreeNode<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match (self, other) {
            (TreeNode::NonLeaf(_), TreeNode::Leaf(_)) => Ordering::Greater,
            (TreeNode::NonLeaf(a), TreeNode::NonLeaf(b)) => a.value.cmp(&b.value),
            (TreeNode::Leaf(a), TreeNode::Leaf(b)) => a.cmp(b),
            _ => Ordering::Less,
        })
    }
}
impl<'a, T: TreeValue> Ord for TreeNode<'a, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl<'a, T: TreeValue> TreeNode<'a, T> {
    pub fn new_leaf(t: T) -> Self {
        TreeNode::Leaf(t)
    }
    pub fn new_nonleaf(s: &'a str, childs: Vec<Self>) -> Self {
        TreeNode::NonLeaf(NonLeaf { value: s, childs })
    }
}

pub struct TreeFmtOption {
    is_done: Vec<bool>,
}
impl<'a, T: TreeValue> TreeNode<'a, T> {}

pub trait TreeFormatter<'a, T: TreeValue, W: Write>: Sized {
    fn fmt_leaf(&mut self, f: &mut W, t: &T) -> Result;
    fn fmt_nonleaf(&mut self, f: &mut W, t: &str) -> Result;

    #[doc(hidden)]
    fn fmt_lists(
        &mut self,
        f: &mut W,
        list: &mut Vec<TreeNode<'a, T>>,
        opt: &mut TreeFmtOption,
    ) -> Result {
        if list.len() == 0 {
            panic!("非葉節點至少要有一個兒子！");
        }
        list.sort();
        let list_len = list.len();

        for node in list.iter_mut().take(list_len - 1) {
            write!(f, "\n")?;
            self.fmt_with(f, node, opt, false)?;
        }
        write!(f, "\n")?;
        self.fmt_with(f, list.last_mut().unwrap(), opt, true)?;
        Ok(())
    }
    #[doc(hidden)]
    fn fmt_with(
        &mut self,
        f: &mut W,
        node: &mut TreeNode<'a, T>,
        opt: &mut TreeFmtOption,
        self_is_end: bool,
    ) -> Result {
        for is_done in opt.is_done.iter().take(opt.is_done.len() - 1) {
            if *is_done {
                write!(f, "    ")?;
            } else {
                write!(f, "│   ")?;
            }
        }
        if !self_is_end {
            write!(f, "├── ")?;
        } else {
            write!(f, "└── ")?;
            *opt.is_done.last_mut().unwrap() = true;
        }
        match node {
            TreeNode::Leaf(leaf) => {
                self.fmt_leaf(f, leaf)?;
            }
            TreeNode::NonLeaf(node) => {
                opt.is_done.push(self_is_end);
                self.fmt_nonleaf(f, node.value)?;
                self.fmt_lists(f, &mut node.childs, opt)?;
                opt.is_done.pop();
            }
        }
        Ok(())
    }
    fn fmt(&mut self, f: &mut W, node: &mut TreeNode<'a, T>) -> Result {
        match node {
            TreeNode::Leaf(leaf) => self.fmt_leaf(f, leaf),
            TreeNode::NonLeaf(node) => {
                self.fmt_nonleaf(f, node.value)?;
                self.fmt_lists(
                    f,
                    &mut node.childs,
                    &mut TreeFmtOption {
                        is_done: vec![false],
                    },
                )
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    impl TreeValue for i32 {}
    type T = TreeNode<'static, i32>;
    fn l(t: i32) -> T {
        TreeNode::new_leaf(t)
    }
    fn n(s: &'static str, childs: Vec<T>) -> T {
        TreeNode::new_nonleaf(s, childs)
    }

    struct Fmter {
        counter: i32,
    }
    impl<W: Write> TreeFormatter<'static, i32, W> for Fmter {
        fn fmt_leaf(&mut self, f: &mut W, t: &i32) -> Result {
            self.counter += 1;
            write!(f, "{} {}", t, self.counter)
        }
        fn fmt_nonleaf(&mut self, f: &mut W, t: &str) -> Result {
            self.counter += 2;
            write!(f, "{}_{}", t, self.counter)
        }
    }
    #[test]
    fn test_display_tree() {
        let mut root = n(
            "aaa",
            vec![
                n("xxx", vec![n("yyy", vec![l(2), n("zzz", vec![l(7)])])]),
                l(4),
                n(
                    "bbb",
                    vec![
                        n("eee", vec![l(8)]),
                        n("ccc", vec![l(5), l(3), n("ddd", vec![l(6)])]),
                    ],
                ),
                l(1),
            ],
        );
        let mut fmter = Fmter { counter: 0 };

        let ans = "
aaa_2
├── 1 3
├── 4 4
├── bbb_6
│   ├── ccc_8
│   │   ├── 3 9
│   │   ├── 5 10
│   │   └── ddd_12
│   │       └── 6 13
│   └── eee_15
│       └── 8 16
└── xxx_18
    └── yyy_20
        ├── 2 21
        └── zzz_23
            └── 7 24"
            .trim();
        let mut v8 = Vec::<u8>::new();
        fmter.fmt(&mut v8, &mut root).unwrap();
        assert_eq!(std::str::from_utf8(&v8).unwrap(), ans);
    }
}
