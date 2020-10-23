use crate::error::Result;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::io::Write;

pub trait TreeValue<'b> {
    fn tree_cmp(&self, other: &Self) -> Ordering;
    fn display_key(&self) -> Cow<'b, str>;
}

pub type Childs<'a, T> = HashMap<(bool, Cow<'a, str>), TreeNode<'a, T>>;
pub enum TreeNode<'a, T: TreeValue<'a>> {
    Leaf(T),
    NonLeaf {
        value: &'a str,
        childs: Childs<'a, T>,
    },
}
impl<'a, T: TreeValue<'a>> Debug for TreeNode<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            TreeNode::Leaf(leaf) => write!(f, "[{}]", leaf.display_key()),
            TreeNode::NonLeaf { value, childs } => write!(f, "`{}` => {:?}", value, childs),
        }
    }
}
impl<'a, T: TreeValue<'a>> TreeNode<'a, T> {
    pub fn new_leaf(t: T) -> Self {
        TreeNode::Leaf(t)
    }
    pub fn key(&self) -> Cow<'a, str> {
        match self {
            TreeNode::Leaf(leaf) => leaf.display_key(),
            TreeNode::NonLeaf { value, .. } => Cow::Borrowed(value),
        }
    }
    fn get_child_map(&mut self) -> &mut Childs<'a, T> {
        match self {
            TreeNode::NonLeaf { childs, .. } => childs,
            _ => panic!("試圖對葉節點 {:?} 取兒子", self),
        }
    }
    fn next_nonleaf<'s>(map: &'s mut Childs<'a, T>, key: &'a str) -> &'s mut Self {
        map.entry((false, Cow::Borrowed(key)))
            .or_insert_with(|| TreeNode::NonLeaf {
                value: key,
                childs: Default::default(),
            })
    }
    pub fn insert_to_map(map: &mut Childs<'a, T>, path: &[&'a str], child: Self) {
        if path.len() == 0 {
            map.insert((true, child.key()), child);
        } else {
            let e = Self::next_nonleaf(map, path[0]);
            e.insert(&path[1..], child);
        }
    }
    pub fn insert(&mut self, path: &[&'a str], leaf: Self) {
        let mut cur = self;
        for p in path {
            let childs = cur.get_child_map();
            cur = Self::next_nonleaf(childs, p);
        }
        let childs = cur.get_child_map();
        childs.insert((true, leaf.key()), leaf);
    }
    pub fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (TreeNode::NonLeaf { .. }, TreeNode::Leaf(_)) => Ordering::Greater,
            (TreeNode::NonLeaf { value: a, .. }, TreeNode::NonLeaf { value: b, .. }) => a.cmp(b),
            (TreeNode::Leaf(a), TreeNode::Leaf(b)) => a.tree_cmp(b),
            _ => Ordering::Less,
        }
    }
}

#[derive(Debug)]
pub struct TreeFmtOption {
    is_done: Vec<bool>,
}

pub trait TreeFormatter<'a, T: TreeValue<'a>, W: Write> {
    fn fmt_leaf(&mut self, f: &mut W, t: &T) -> Result;
    fn fmt_nonleaf(&mut self, f: &mut W, t: &str) -> Result;

    #[doc(hidden)]
    fn fmt_lists(&mut self, f: &mut W, map: &mut Childs<'a, T>, opt: &mut TreeFmtOption) -> Result {
        if map.len() == 0 {
            panic!("非葉節點至少要有一個兒子！");
        }
        let mut list: Vec<_> = map.iter_mut().map(|(_, v)| v).collect();
        list.sort_by(|a, b| a.cmp(&b));
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
        log::trace!(
            "打印節點 {:?}: opt={:?}, self_is_end={}",
            node,
            opt,
            self_is_end
        );
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
            TreeNode::NonLeaf { value, childs } => {
                opt.is_done.push(false);
                self.fmt_nonleaf(f, value)?;
                self.fmt_lists(f, childs, opt)?;
                opt.is_done.pop();
            }
        }
        Ok(())
    }
    fn fmt(&mut self, f: &mut W, node: &mut TreeNode<'a, T>) -> Result {
        match node {
            TreeNode::Leaf(leaf) => self.fmt_leaf(f, leaf),
            TreeNode::NonLeaf { value, childs } => {
                self.fmt_nonleaf(f, value)?;
                self.fmt_lists(
                    f,
                    childs,
                    &mut TreeFmtOption {
                        is_done: vec![false],
                    },
                )
            }
        }
    }
    fn fmt_all(&mut self, f: &mut W, forest: impl Iterator<Item = TreeNode<'a, T>>) -> Result {
        for mut root in forest {
            self.fmt(f, &mut root)?;
            writeln!(f, "")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    impl<'a> TreeValue<'a> for &'a String {
        fn tree_cmp(&self, other: &Self) -> Ordering {
            self.cmp(other)
        }
        fn display_key(&self) -> Cow<'a, str> {
            Cow::Borrowed(self)
        }
    }
    type T<'a> = TreeNode<'a, &'a String>;
    fn l<'a>(t: &'a String) -> T<'a> {
        TreeNode::new_leaf(t)
    }
    fn n<'a>(s: &'a str, childs: Vec<T<'a>>) -> T<'a> {
        let mut map = HashMap::new();
        for child in childs.into_iter() {
            let is_leaf = match child {
                TreeNode::Leaf(_) => true,
                _ => false,
            };
            let key = child.key();
            map.insert((is_leaf, key), child);
        }
        TreeNode::NonLeaf {
            value: s,
            childs: map,
        }
    }

    struct Fmter {
        counter: i32,
    }
    impl<'a, W: Write> TreeFormatter<'a, &'a String, W> for Fmter {
        fn fmt_leaf(&mut self, f: &mut W, t: &&String) -> Result {
            self.counter += 1;
            write!(f, "{} {}", t, self.counter)?;
            Ok(())
        }
        fn fmt_nonleaf(&mut self, f: &mut W, t: &str) -> Result {
            self.counter += 2;
            write!(f, "{}_{}", t, self.counter)?;
            Ok(())
        }
    }
    #[test]
    fn test_display_tree() {
        let v1 = vec!["0", "1", "2", "3", "4", "5", "6", "7", "8"];
        let v: Vec<String> = v1.iter().map(|s| s.to_string()).collect();
        let mut root = n(
            "aaa",
            vec![
                n(
                    "xxx",
                    vec![n("yyy", vec![l(&v[2]), n("zzz", vec![l(&v[7])])])],
                ),
                l(&v[4]),
                n(
                    "bbb",
                    vec![
                        n("eee", vec![l(&v[8])]),
                        n("ccc", vec![l(&v[5]), l(&v[3]), n("ddd", vec![l(&v[6])])]),
                    ],
                ),
                l(&v[1]),
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
