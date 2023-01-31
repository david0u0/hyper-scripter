use crate::error::Result;
use fxhash::FxHashMap as HashMap;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::result::Result as StdResult;
use unicode_width::UnicodeWidthStr;

pub trait TreeValue<'b> {
    type CmpKey: Ord + Copy;
    fn cmp_key(&self) -> Self::CmpKey;
    fn display_key(&self) -> Cow<'b, str>;
}

/// bool = is_leaf
pub type Childs<'a, T, K> = HashMap<(bool, Cow<'a, str>), TreeNode<'a, T, K>>;
pub struct NonLeafInner<'a, T: TreeValue<'a>, K> {
    max_cmp_key: Option<K>,
    value: &'a str,
    childs: Childs<'a, T, K>,
}
pub enum TreeNode<'a, T: TreeValue<'a>, K> {
    Leaf(T),
    NonLeaf(NonLeafInner<'a, T, K>),
}
impl<'a, T: TreeValue<'a>, K> Debug for TreeNode<'a, T, K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            TreeNode::Leaf(leaf) => write!(f, "[{}]", leaf.display_key()),
            TreeNode::NonLeaf(NonLeafInner {
                value,
                childs,
                max_cmp_key: _,
            }) => write!(f, "`{}` => {:?}", value, childs),
        }
    }
}
impl<'a, T: TreeValue<'a, CmpKey = K>, K: Ord + Copy> NonLeafInner<'a, T, K> {
    fn insert<I: Iterator<Item = &'a str>>(
        &mut self,
        mut path: I,
        leaf: TreeNode<'a, T, K>,
    ) -> Option<K> {
        let cmp_key;
        if let Some(p) = path.next() {
            let e = TreeNode::next_nonleaf(&mut self.childs, p);
            cmp_key = e.insert(path, leaf);
        } else {
            cmp_key = leaf.cmp_key();
            self.childs.insert((true, leaf.display_key()), leaf);
        }

        let mut need_change = false;
        if let Some(cmp_key) = cmp_key {
            if let Some(self_cmp_key) = self.max_cmp_key {
                need_change = cmp_key > self_cmp_key;
            } else {
                need_change = true;
            }
        }
        if need_change {
            self.max_cmp_key = cmp_key;
            self.max_cmp_key
        } else {
            None
        }
    }
}
impl<'a, T: TreeValue<'a, CmpKey = K>, K: Ord + Copy> TreeNode<'a, T, K> {
    pub fn new_leaf(t: T) -> Self {
        TreeNode::Leaf(t)
    }
    pub fn new_nonleaf(value: &'a str, childs: Childs<'a, T, K>) -> Self {
        TreeNode::NonLeaf(NonLeafInner {
            value,
            childs,
            max_cmp_key: None,
        })
    }
    fn next_nonleaf<'s>(
        map: &'s mut Childs<'a, T, K>,
        key: &'a str,
    ) -> &'s mut NonLeafInner<'a, T, K> {
        let e = map.entry((false, Cow::Borrowed(key))).or_insert_with(|| {
            TreeNode::NonLeaf(NonLeafInner {
                value: key,
                max_cmp_key: None,
                childs: Default::default(),
            })
        });
        match e {
            TreeNode::NonLeaf(t) => t,
            _ => unreachable!(),
        }
    }
    pub fn display_key(&self) -> Cow<'a, str> {
        match self {
            TreeNode::Leaf(leaf) => leaf.display_key(),
            TreeNode::NonLeaf(t) => Cow::Borrowed(t.value),
        }
    }
    pub fn cmp_key(&self) -> Option<K> {
        match self {
            TreeNode::Leaf(t) => Some(t.cmp_key()),
            TreeNode::NonLeaf(t) => t.max_cmp_key,
        }
    }
    pub fn insert_to_map<I: Iterator<Item = &'a str>>(
        map: &mut Childs<'a, T, K>,
        mut path: I,
        child: Self,
    ) {
        if let Some(p) = path.next() {
            let e = Self::next_nonleaf(map, p);
            e.insert(path, child);
        } else {
            map.insert((true, child.display_key()), child);
        }
    }
    // NOTE: 取名 cmp 的話，clippy 會叫你實作 Ord，很麻煩
    pub fn simple_cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (TreeNode::NonLeaf(_), TreeNode::Leaf(_)) => Ordering::Greater,
            (TreeNode::Leaf(_), TreeNode::NonLeaf(_)) => Ordering::Less,
            _ => match (self.cmp_key(), other.cmp_key()) {
                (None, None) => Ordering::Equal,
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(&b),
            },
        }
    }
}

#[derive(Debug)]
pub struct TreeFmtOption {
    is_done: Vec<bool>,
}

pub enum LeadingDisplay<'a> {
    Some {
        opt: &'a TreeFmtOption,
        self_is_end: bool,
    },
    None,
}

enum DisplayType {
    One,
    Two,
    Three,
    Four,
}
impl DisplayType {
    const fn as_str(&self) -> &'static str {
        match self {
            DisplayType::One => "   ",
            DisplayType::Two => "│  ",
            DisplayType::Three => "├── ",
            DisplayType::Four => "└── ",
        }
    }
}

impl<'a> LeadingDisplay<'a> {
    fn visit<E, F: FnMut(DisplayType) -> StdResult<(), E>>(
        &self,
        mut visitor: F,
    ) -> StdResult<(), E> {
        if let LeadingDisplay::Some { opt, self_is_end } = self {
            for is_done in opt.is_done.iter().take(opt.is_done.len() - 1) {
                if *is_done {
                    visitor(DisplayType::One)?;
                } else {
                    visitor(DisplayType::Two)?;
                }
            }
            if !self_is_end {
                visitor(DisplayType::Three)?;
            } else {
                visitor(DisplayType::Four)?;
            }
        }
        Ok(())
    }
    pub fn width(&self) -> usize {
        let mut w = 0;
        let inner = |ty: DisplayType| -> StdResult<(), ()> {
            w += ty.as_str().width();
            Ok(())
        };
        self.visit(inner).unwrap();
        w
    }
}

impl Display for LeadingDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let inner = |ty: DisplayType| -> FmtResult { write!(f, "{}", ty.as_str()) };
        self.visit(inner)
    }
}

pub trait TreeFormatter<'a, T: TreeValue<'a, CmpKey = K>, K: Ord + Copy> {
    fn fmt_leaf(&mut self, leading: LeadingDisplay<'_>, t: &T) -> Result;
    fn fmt_nonleaf(&mut self, leading: LeadingDisplay<'_>, t: &str) -> Result;

    #[doc(hidden)]
    fn fmt_lists(&mut self, map: &mut Childs<'a, T, K>, opt: &mut TreeFmtOption) -> Result {
        if map.is_empty() {
            panic!("非葉節點至少要有一個兒子！");
        }
        let mut list: Vec<_> = map.iter_mut().map(|(_, v)| v).collect();
        list.sort_by(|a, b| a.simple_cmp(b));
        let list_len = list.len();

        for node in list.iter_mut().take(list_len - 1) {
            self.fmt_with(node, opt, false)?;
        }
        self.fmt_with(list.last_mut().unwrap(), opt, true)?;
        Ok(())
    }
    #[doc(hidden)]
    fn fmt_with(
        &mut self,
        node: &mut TreeNode<'a, T, K>,
        opt: &mut TreeFmtOption,
        self_is_end: bool,
    ) -> Result {
        log::trace!(
            "打印節點 {:?}: opt={:?}, self_is_end={}",
            node,
            opt,
            self_is_end
        );
        let leading = LeadingDisplay::Some { opt, self_is_end };
        match node {
            TreeNode::Leaf(leaf) => {
                self.fmt_leaf(leading, leaf)?;
                if self_is_end {
                    *opt.is_done.last_mut().unwrap() = true;
                }
            }
            TreeNode::NonLeaf(NonLeafInner { value, childs, .. }) => {
                self.fmt_nonleaf(leading, value)?;
                if self_is_end {
                    *opt.is_done.last_mut().unwrap() = true;
                }
                opt.is_done.push(false);
                self.fmt_lists(childs, opt)?;
                opt.is_done.pop();
            }
        }
        Ok(())
    }
    fn fmt(&mut self, node: &mut TreeNode<'a, T, K>) -> Result {
        match node {
            TreeNode::Leaf(leaf) => self.fmt_leaf(LeadingDisplay::None, leaf),
            TreeNode::NonLeaf(NonLeafInner { value, childs, .. }) => {
                self.fmt_nonleaf(LeadingDisplay::None, value)?;
                self.fmt_lists(
                    childs,
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
    impl<'a> TreeValue<'a> for &'a String {
        type CmpKey = &'a String;
        fn cmp_key(&self) -> &'a String {
            self
        }
        fn display_key(&self) -> Cow<'a, str> {
            Cow::Borrowed(self)
        }
    }
    type T<'a> = TreeNode<'a, &'a String, &'a String>;
    fn l<'a>(t: &'a String) -> T<'a> {
        TreeNode::new_leaf(t)
    }
    fn n<'a>(s: &'a str, childs: Vec<T<'a>>) -> T<'a> {
        let mut non_leaf = NonLeafInner {
            max_cmp_key: None,
            value: s,
            childs: Default::default(),
        };
        for child in childs.into_iter() {
            non_leaf.insert(std::iter::empty(), child);
        }
        TreeNode::NonLeaf(non_leaf)
    }

    #[derive(Default)]
    struct Fmter {
        counter: i32,
        s: String,
    }
    impl<'a> TreeFormatter<'a, &'a String, &'a String> for Fmter {
        fn fmt_leaf(&mut self, l: LeadingDisplay, t: &&String) -> Result {
            self.counter += 1;
            self.s += &format!("{}{} {}\n", l, t, self.counter);
            Ok(())
        }
        fn fmt_nonleaf(&mut self, l: LeadingDisplay, t: &str) -> Result {
            self.counter += 2;
            self.s += &format!("{}{}_{}\n", l, t, self.counter);
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
        let mut fmter = Fmter::default();

        let ans = "
aaa_2
├── 1 3
├── 4 4
├── xxx_6
│  └── yyy_8
│     ├── 2 9
│     └── zzz_11
│        └── 7 12
└── bbb_14
   ├── ccc_16
   │  ├── 3 17
   │  ├── 5 18
   │  └── ddd_20
   │     └── 6 21
   └── eee_23
      └── 8 24
"
        .trim_start();
        fmter.fmt(&mut root).unwrap();
        assert_eq!(fmter.s, ans);
    }
}
