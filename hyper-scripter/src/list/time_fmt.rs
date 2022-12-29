use crate::script_time::ScriptTime;
use crate::state::State;
use chrono::{Datelike, Local, NaiveDateTime, TimeZone};

static NOW: State<NaiveDateTime> = State::new();

pub fn init() {
    let now = Local::now().naive_local();
    log::debug!("now = {:?}", now);
    NOW.set(now);
}

// TODO: return something impl Display?
pub fn fmt<T>(time: &ScriptTime<T>) -> String {
    let time = Local.from_utc_datetime(&**time).naive_local();
    let now = NOW.get();
    log::debug!("time = {:?}", time);

    if now.date() == time.date() {
        format!("{}", time.format("%H:%M"))
    } else if now.year() == time.year() {
        format!("{}", time.format("%d %b"))
    } else {
        format!("{}", time.format("%Y"))
    }
}
