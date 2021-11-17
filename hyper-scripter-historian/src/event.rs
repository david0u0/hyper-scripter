use chrono::NaiveDateTime;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Exec,
    ExecDone,
    Read,
    Write,
    Miss,
}

#[derive(Debug)]
pub enum EventData<'a> {
    Exec {
        content: &'a str,
        args: &'a str,
        dir: Option<&'a Path>,
    },
    ExecDone {
        code: i32,
        main_event_id: i64,
    },
    Read,
    Write,
    Miss,
}

impl EventData<'_> {
    pub fn get_type(&self) -> EventType {
        match self {
            EventData::Exec { .. } => EventType::Exec,
            EventData::ExecDone { .. } => EventType::ExecDone,
            EventData::Read => EventType::Read,
            EventData::Write => EventType::Write,
            EventData::Miss => EventType::Miss,
        }
    }
}

impl EventType {
    pub const fn get_code(&self) -> i8 {
        use EventType::*;
        match self {
            Read => 0,
            Write => 1,
            Miss => 2,
            Exec => 3,
            ExecDone => 4,
        }
    }
}

#[derive(Debug)]
pub struct Event<'a> {
    pub data: EventData<'a>,
    pub script_id: i64,
    pub time: NaiveDateTime,
}
