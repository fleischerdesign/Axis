use super::provider::{Task, TaskList, TaskProvider};
use super::utils::{data_dir, load_json, save_json};
use log::info;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
struct Storage {
    tasks: Vec<LocalTask>,
    next_id: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct LocalTask {
    id: u64,
    title: String,
    done: bool,
}

pub struct LocalTodoProvider {
    path: PathBuf,
    storage: Storage,
}

impl LocalTodoProvider {
    pub fn new() -> Self {
        let dir = data_dir().unwrap_or_else(|| PathBuf::from("."));
        let path = dir.join("tasks.json");
        let storage = load_json(&path);
        Self { path, storage }
    }

    fn save(&self) {
        save_json(&self.path, &self.storage);
    }
}

impl TaskProvider for LocalTodoProvider {
    fn name(&self) -> &str {
        "Lokal"
    }

    fn icon(&self) -> &str {
        "checkbox-checked-symbolic"
    }

    fn lists(&mut self) -> Result<Vec<TaskList>, String> {
        Ok(vec![TaskList {
            id: "default".to_string(),
            title: "Aufgaben".to_string(),
        }])
    }

    fn tasks(&mut self, _list_id: &str) -> Result<Vec<Task>, String> {
        Ok(self
            .storage
            .tasks
            .iter()
            .map(|t| Task {
                id: t.id.to_string(),
                title: t.title.clone(),
                done: t.done,
                provider: "local".to_string(),
            })
            .collect())
    }

    fn add_task(&mut self, _list_id: &str, title: &str) -> Result<Task, String> {
        let id = self.storage.next_id;
        self.storage.next_id += 1;
        let task = LocalTask {
            id,
            title: title.to_string(),
            done: false,
        };
        self.storage.tasks.push(task.clone());
        self.save();

        info!("[local-tasks] Added: {}", title);

        Ok(Task {
            id: id.to_string(),
            title: task.title,
            done: false,
            provider: "local".to_string(),
        })
    }

    fn toggle_task(&mut self, _list_id: &str, task_id: &str, done: bool) -> Result<(), String> {
        let id: u64 = task_id.parse().map_err(|_| "Invalid task ID")?;
        if let Some(task) = self.storage.tasks.iter_mut().find(|t| t.id == id) {
            task.done = done;
            self.save();
            info!("[local-tasks] Toggled {} -> {}", task_id, done);
            Ok(())
        } else {
            Err("Task not found".to_string())
        }
    }
}
