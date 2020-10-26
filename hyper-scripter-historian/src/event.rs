#[derive(Debug, Display, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Exec,
    ExecDone,
    Read,
    Miss,
}

#[derive(Debug)]
pub enum EventData<'a> {
    Exec(&'a str),
    ExecDone(i32),
    Read,
    Miss,
}

impl EventData<'_> {
    pub fn get_type(&self) -> EventType {
        match self {
            EventData::Exec(_) => EventType::Exec,
            EventData::ExecDone(_) => EventType::ExecDone,
            EventData::Read => EventType::Read,
            EventData::Miss => EventType::Miss,
        }
    }
}

#[derive(Debug)]
pub struct Event<'a> {
    pub data: EventData<'a>,
    pub script_id: i64,
}
