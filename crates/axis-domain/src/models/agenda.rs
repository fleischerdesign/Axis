use crate::models::calendar::CalendarEvent;
use crate::models::tasks::{Task, TaskList};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgendaStatus {
    pub events: Vec<CalendarEvent>,
    pub tasks: Vec<Task>,
    pub task_lists: Vec<TaskList>,
    pub selected_list_id: Option<String>,
    pub is_loading_tasks: bool,
    pub is_loading_events: bool,
}
