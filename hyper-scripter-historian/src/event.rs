use chrono::NaiveDateTime;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Exec,
    ExecDone,
    Read,
    Write,
}

#[derive(Debug)]
pub enum EventData<'a> {
    Exec {
        content: &'a str,
        args: &'a str,
        path: &'a Path,
    },
    ExecDone {
        code: i32,
        main_event_id: i64,
    },
    Read,
    Write,
}

impl EventData<'_> {
    pub fn get_type(&self) -> EventType {
        match self {
            EventData::Exec { .. } => EventType::Exec,
            EventData::ExecDone { .. } => EventType::ExecDone,
            EventData::Read => EventType::Read,
            EventData::Write => EventType::Write,
        }
    }
}

impl EventType {
    pub const fn get_str(&self) -> &'static str {
        use EventType::*;
        match self {
            ExecDone => "ExecDone",
            Exec => "Exec",
            Read => "Read",
            Write => "Write",
        }
    }
}

#[derive(Debug)]
pub struct Event<'a> {
    pub data: EventData<'a>,
    pub script_id: i64,
    pub time: NaiveDateTime,
}
