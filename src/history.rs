use crate::script::{ScriptInfo, ScriptName};
use crate::tag::TagFilters;
use std::collections::{hash_map::Entry, HashMap};

#[derive(Default, Debug)]
pub struct History<'a> {
    map: HashMap<String, ScriptInfo<'a>>,
    latest_name: Option<String>,
}

impl<'a> History<'a> {
    pub fn iter(&self) -> impl Iterator<Item = &ScriptInfo> {
        self.map.iter().map(|(_, info)| info)
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ScriptInfo<'a>> {
        self.map.iter_mut().map(|(_, info)| info)
    }
    pub fn into_iter(self) -> impl Iterator<Item = ScriptInfo<'a>> {
        self.map.into_iter().map(|(_, info)| info)
    }
    fn latest_mut_no_cache(&mut self) -> Option<&mut ScriptInfo<'a>> {
        let latest = self.map.iter_mut().max_by_key(|(_, info)| info.last_time());
        if let Some((name, info)) = latest {
            self.latest_name = Some(name.clone());
            Some(info)
        } else {
            None
        }
    }
    pub fn latest_mut(&mut self) -> Option<&mut ScriptInfo<'a>> {
        if let Some(name) = &self.latest_name {
            // FIXME: 一旦 rust nll 進化就修掉這段
            if self.map.contains_key(name) {
                return self.map.get_mut(name);
            }
            log::warn!("快取住的最新資訊已經不見了…？重找一次");
        }
        self.latest_mut_no_cache()
    }
    pub fn new<I: Iterator<Item = ScriptInfo<'a>>>(iter: I) -> Self {
        let mut map = HashMap::new();
        for s in iter {
            map.insert(s.name.key().into_owned(), s);
        }
        History {
            map,
            latest_name: None,
        }
    }
    pub fn get_mut(&mut self, name: &ScriptName) -> Option<&mut ScriptInfo<'a>> {
        self.map.get_mut(&*name.key())
    }
    pub fn remove(&mut self, name: &ScriptName) {
        self.map.remove(&*name.key());
    }
    pub fn insert(&mut self, info: ScriptInfo<'a>) {
        self.map.insert(info.name.key().into_owned(), info);
    }
    pub fn entry(&mut self, name: &ScriptName) -> Entry<'_, String, ScriptInfo<'a>> {
        self.map.entry(name.key().into_owned())
    }
    pub fn filter_by_group(&mut self, filter: &TagFilters) {
        // TODO: 優化
        let map: HashMap<_, _> = self
            .map
            .drain()
            .filter(|(_, info)| filter.filter(&info.tags))
            .collect();
        self.map = map;
    }
}
