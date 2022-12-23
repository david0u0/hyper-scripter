use crate::script_time::ScriptTime;
use crate::state::State;
use chrono::{Datelike, Local, NaiveDateTime, TimeZone, Utc};

static NOW: State<NaiveDateTime> = State::new();

pub fn init() {
    let now = Utc::now().naive_local();
    NOW.set(now);
}

// TODO: return something impl Display?
pub fn fmt<T>(time: &ScriptTime<T>) -> String {
    let time = Local.from_utc_datetime(&**time).naive_local();
    let now = NOW.get();

    if now.date() == time.date() {
        format!("{}", time.format("%H:%M"))
    } else if now.year() == time.year() {
        format!("{}", time.format("%d %b"))
    } else {
        format!("{}", time.format("%Y"))
    }
}
