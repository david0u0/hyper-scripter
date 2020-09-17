use chrono::{NaiveDateTime, Utc};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, Ord, Eq)]
pub struct ScriptTime {
    changed: bool,
    time: NaiveDateTime,
}
impl PartialEq for ScriptTime {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}
impl PartialOrd for ScriptTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

impl Deref for ScriptTime {
    type Target = NaiveDateTime;
    fn deref(&self) -> &Self::Target {
        &self.time
    }
}

impl DerefMut for ScriptTime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.time
    }
}

impl ScriptTime {
    pub fn now() -> Self {
        ScriptTime {
            time: Utc::now().naive_local(),
            changed: true,
        }
    }
    pub fn new_or(time: Option<NaiveDateTime>, default: NaiveDateTime) -> Self {
        if let Some(time) = time {
            ScriptTime::new(time)
        } else {
            ScriptTime {
                time: default,
                changed: true,
            }
        }
    }
    pub fn new(time: NaiveDateTime) -> Self {
        ScriptTime {
            time,
            changed: false,
        }
    }
    pub fn has_changed(&self) -> bool {
        self.changed
    }
}
