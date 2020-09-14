use crate::error::Result;
use crate::script::{ScriptInfo, ScriptName};
use crate::tag::TagFilterGroup;
use std::collections::HashMap;

pub mod helper;
use helper::*;

pub type ScriptRepoEntry<'a, 'b> = RepoEntry<'a, 'b, ()>;

#[derive(Default, Debug)]
pub struct ScriptRepo<'a> {
    map: HashMap<String, ScriptInfo<'a>>,
    hidden_map: HashMap<String, ScriptInfo<'a>>,
    latest_name: Option<String>,
    env: (),
}

impl Environment for () {
    fn handle_change(&self, _info: &ScriptInfo) -> Result {
        Ok(())
    }
}

impl<'a> ScriptRepo<'a> {
    pub fn iter(&self) -> impl Iterator<Item = &ScriptInfo> {
        self.map.iter().map(|(_, info)| info)
    }
    pub fn iter_mut<'b>(&'b mut self) -> Iter<'a, 'b, ()> {
        Iter {
            iter: self.map.iter_mut(),
            env: &self.env,
        }
    }
    // fn latest_mut_no_cache(&mut self) -> Option<&mut ScriptInfo<'a>> {
    //     let latest = self.map.iter_mut().max_by_key(|(_, info)| info.last_time());
    //     if let Some((name, info)) = latest {
    //         self.latest_name = Some(name.clone());
    //         Some(info)
    //     } else {
    //         None
    //     }
    // }
    pub fn latest_mut(&mut self, n: usize) -> Option<ScriptRepoEntry<'a, '_>> {
        // if let Some(name) = &self.latest_name {
        //     // FIXME: 一旦 rust nll 進化就修掉這段
        //     if self.map.contains_key(name) {
        //         return self.map.get_mut(name);
        //     }
        //     log::warn!("快取住的最新資訊已經不見了…？重找一次");
        // }
        // self.latest_mut_no_cache()
        let mut v: Vec<_> = self.map.iter_mut().map(|(_, s)| s).collect();
        v.sort_by_key(|s| s.last_time());
        if v.len() >= n {
            // SAFETY: 從向量中讀一個可變指針安啦
            let t = unsafe { std::ptr::read(&v[v.len() - n]) };
            Some(RepoEntry {
                info: t,
                env: &self.env,
            })
        } else {
            None
        }
    }
    pub fn new<I: Iterator<Item = ScriptInfo<'a>>>(iter: I) -> Self {
        let mut map = HashMap::new();
        for mut s in iter {
            s.tags.sort();
            map.insert(s.name.key().into_owned(), s);
        }
        ScriptRepo {
            map,
            hidden_map: Default::default(),
            latest_name: None,
            env: (),
        }
    }
    pub fn get_mut(&mut self, name: &ScriptName) -> Option<ScriptRepoEntry<'a, '_>> {
        match self.map.get_mut(&*name.key()) {
            None => None,
            Some(info) => Some(RepoEntry {
                info,
                env: &self.env,
            }),
        }
    }
    pub fn get_hidden_mut(&mut self, name: &ScriptName) -> Option<&mut ScriptInfo<'a>> {
        self.hidden_map.get_mut(&*name.key())
    }
    pub fn remove(&mut self, name: &ScriptName) {
        self.map.remove(&*name.key());
    }
    pub fn insert(&mut self, info: ScriptInfo<'a>) {
        self.map.insert(info.name.key().into_owned(), info);
    }
    pub fn upsert<F: FnOnce() -> ScriptInfo<'a>>(
        &mut self,
        name: &ScriptName,
        default: F,
    ) -> ScriptRepoEntry<'a, '_> {
        let info = self
            .map
            .entry(name.key().into_owned())
            .or_insert_with(default);
        RepoEntry {
            info,
            env: &self.env,
        }
    }
    pub fn filter_by_tag(&mut self, filter: &TagFilterGroup) {
        // TODO: 優化
        log::debug!("根據標籤 {:?} 進行篩選", filter);
        let drain = self.map.drain();
        let mut map = HashMap::new();
        for (key, info) in drain {
            if filter.filter(&info.tags) {
                log::trace!("腳本 {:?} 通過篩選", info.name);
                map.insert(key, info);
            } else {
                log::trace!("掰掰，{:?}", info.name);
                self.hidden_map.insert(key, info);
            }
        }
        self.map = map;
    }
}
