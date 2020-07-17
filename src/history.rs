use crate::error::Result;
use crate::script::ScriptName;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptHistory {
    pub edit_time: DateTime<Utc>,
    pub exec_time: Option<DateTime<Utc>>,
    pub name: ScriptName,
    pub last_edit_path: PathBuf,
}

impl ScriptHistory {
    pub fn last_time(&self) -> DateTime<Utc> {
        if let Some(exec_time) = self.exec_time {
            std::cmp::max(self.edit_time, exec_time)
        } else {
            self.edit_time
        }
    }
    pub fn new(name: ScriptName) -> Result<Self> {
        Ok(ScriptHistory {
            name,
            last_edit_path: std::env::current_dir()?,
            edit_time: Utc::now(),
            exec_time: None,
        })
    }
}
