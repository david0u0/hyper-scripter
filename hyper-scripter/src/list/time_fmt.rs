use crate::script_time::ScriptTime;
use crate::state::State;
use chrono::{Datelike, Local, NaiveDateTime, TimeZone};
use std::fmt::{Display, Formatter, Result as FmtResult};

static NOW: State<NaiveDateTime> = State::new();

pub fn init() {
    let now = Local::now().naive_local();
    log::debug!("now = {:?}", now);
    NOW.set(now);
}

pub struct DisplayTime(NaiveDateTime);
impl Display for DisplayTime {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        let time = &self.0;
        let now = NOW.get();
        log::debug!("time = {:?}", time);

        let diff = *now - *time;
        if diff.num_hours() < 12 || now.date() == time.date() {
            write!(w, "{}", time.format("%H:%M"))
        } else if diff.num_days() < 180 || now.year() == time.year() {
            write!(w, "{}", time.format("%d %b"))
        } else {
            write!(w, "{}", time.format("%Y"))
        }
    }
}

pub fn fmt<T>(time: &ScriptTime<T>) -> DisplayTime {
    let time = Local.from_utc_datetime(&**time).naive_local();
    DisplayTime(time)
}
