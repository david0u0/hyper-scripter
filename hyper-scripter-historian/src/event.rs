use chrono::NaiveDateTime;

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Exec,
    ExecDone,
    Read,
}

#[derive(Debug)]
pub enum EventData<'a> {
    Exec { content: &'a str, args: &'a str },
    ExecDone { code: i32, main_event_id: i64 },
    Read,
}

impl EventData<'_> {
    pub fn get_type(&self) -> EventType {
        match self {
            EventData::Exec { .. } => EventType::Exec,
            EventData::ExecDone { .. } => EventType::ExecDone,
            EventData::Read => EventType::Read,
        }
    }
}

#[derive(Debug)]
pub struct Event<'a> {
    pub data: EventData<'a>,
    pub script_id: i64,
    pub time: NaiveDateTime,
}
