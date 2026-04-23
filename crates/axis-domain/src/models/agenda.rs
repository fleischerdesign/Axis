use serde::{Deserialize, Serialize};
use crate::models::calendar::CalendarEvent;
use crate::models::tasks::Task;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgendaStatus {
    pub events: Vec<CalendarEvent>,
    pub tasks: Vec<Task>,
}

impl Default for AgendaStatus {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            tasks: Vec::new(),
        }
    }
}
