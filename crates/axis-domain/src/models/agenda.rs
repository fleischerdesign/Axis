use serde::{Deserialize, Serialize};
use crate::models::calendar::CalendarEvent;
use crate::models::tasks::{Task, TaskList};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgendaStatus {
    pub events: Vec<CalendarEvent>,
    pub tasks: Vec<Task>,
    pub task_lists: Vec<TaskList>,
    pub selected_list_id: Option<String>,
    pub is_loading_tasks: bool,
    pub is_loading_events: bool,
}

impl Default for AgendaStatus {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            tasks: Vec::new(),
            task_lists: Vec::new(),
            selected_list_id: None,
            is_loading_tasks: false,
            is_loading_events: false,
        }
    }
}
