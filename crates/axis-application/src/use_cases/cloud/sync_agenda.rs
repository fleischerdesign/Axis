use axis_domain::models::agenda::AgendaStatus;
use axis_domain::ports::calendar::CalendarProvider;
use axis_domain::ports::tasks::TaskProvider;
use std::sync::Arc;
use chrono::{Utc, Duration, Datelike, TimeZone};

pub struct SyncAgendaUseCase {
    calendar_provider: Arc<dyn CalendarProvider>,
    task_provider: Arc<dyn TaskProvider>,
}

impl SyncAgendaUseCase {
    pub fn new(
        calendar_provider: Arc<dyn CalendarProvider>,
        task_provider: Arc<dyn TaskProvider>
    ) -> Self {
        Self { calendar_provider, task_provider }
    }

    pub async fn execute(&self, list_id: Option<String>) -> Result<AgendaStatus, String> {
        let now = Utc::now();
        let start = (now - Duration::days(30)).to_rfc3339();
        let end = (now + Duration::days(60)).to_rfc3339();

        let events = self.calendar_provider.get_events(&start, &end).await
            .map_err(|e| e.to_string())?;

        let task_lists = self.task_provider.get_lists().await
            .map_err(|e| e.to_string())?;

        let selected_id = list_id.or_else(|| task_lists.first().map(|l| l.id.clone()));
        
        let mut tasks = Vec::new();
        if let Some(ref id) = selected_id {
            tasks = self.task_provider.get_tasks(id).await
                .map_err(|e| e.to_string())?;
        }

        Ok(AgendaStatus {
            events,
            tasks,
            task_lists,
            selected_list_id: selected_id,
        })
    }
}
