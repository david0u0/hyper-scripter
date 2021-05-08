use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::fmt::Debug;

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
    pub fn new_or(time: Option<NaiveDateTime>, default: Self) -> Self {
        if let Some(time) = time {
            ScriptTime {
                time,
                changed: None,
            }
        } else {
            default
        }
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
