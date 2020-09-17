#[derive(Debug, Display, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Exec,
    Read,
}

#[derive(Debug)]
pub enum EventData {
    Exec(String),
    Read,
}

impl EventData {
    pub fn get_type(&self) -> EventType {
        match self {
            EventData::Exec(_) => EventType::Exec,
            EventData::Read => EventType::Read,
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub data: EventData,
    pub script_id: i64,
}
