use chrono::{NaiveDateTime, Utc};
use std::cmp::{Ordering, PartialEq, PartialOrd};

#[derive(Debug, Clone, Copy, Ord, Eq, Deref)]
pub struct ScriptTime {
    changed: bool,
    #[deref]
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

impl ScriptTime {
    pub fn now() -> Self {
        ScriptTime {
            time: Utc::now().naive_utc(),
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
