use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::fmt::Debug;

/// 可能帶著資料的時間。
/// 如果有資料，代表「這些資料是新的產生，應儲存起來但還未存」
/// 如果沒資料，就視為上次儲存的一個快照，只記得時間就好，因為我們通常不會需要上一次存下的資料
#[derive(Debug, Clone, Deref)]
pub struct ScriptTime<T = ()> {
    changed: Option<T>,
    #[deref]
    time: NaiveDateTime,
}
impl<T> PartialEq for ScriptTime<T> {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}
impl<T> PartialOrd for ScriptTime<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.time.partial_cmp(&other.time)
    }
}
impl<T> Ord for ScriptTime<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl<T> Eq for ScriptTime<T> {}

impl<T> ScriptTime<T> {
    pub fn now(data: T) -> Self {
        ScriptTime {
            time: Utc::now().naive_utc(),
            changed: Some(data),
        }
    }
    pub fn new_or_else<F: FnOnce() -> Self>(time: Option<NaiveDateTime>, default: F) -> Self {
        match time {
            Some(time) => ScriptTime {
                time,
                changed: None,
            },
            None => default(),
        }
    }
    pub fn new_or(time: Option<NaiveDateTime>, default: Self) -> Self {
        ScriptTime::new_or_else(time, || default)
    }
    pub fn new(time: NaiveDateTime) -> Self {
        ScriptTime {
            time,
            changed: None,
        }
    }
    pub fn data(&self) -> Option<&T> {
        self.changed.as_ref()
    }
    pub fn has_changed(&self) -> bool {
        self.changed.is_some()
    }
}

impl<T> std::fmt::Display for ScriptTime<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let local_time = Local.from_utc_datetime(&self.time);
        write!(f, "{}", local_time.format("%Y-%m-%d %H:%M"))
    }
}
