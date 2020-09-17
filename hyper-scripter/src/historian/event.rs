#[derive(Debug, Display, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Exec,
    ExecDone,
    Read,
}

#[derive(Debug)]
pub enum EventData {
    Exec(String),
    ExecDone(i32),
    Read,
}

impl EventData {
    pub fn get_type(&self) -> EventType {
        match self {
            EventData::Exec(_) => EventType::Exec,
            EventData::ExecDone(_) => EventType::ExecDone,
            EventData::Read => EventType::Read,
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub data: EventData,
    pub script_id: i64,
}
