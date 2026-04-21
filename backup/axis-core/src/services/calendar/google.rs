use super::provider::{AuthStatus, CalendarEvent, CalendarProvider};
use crate::services::google::GoogleAuthRegistry;
use crate::services::tasks::utils::{api_get, build_http_client};
use serde::Deserialize;

const CALENDAR_SCOPE: &[&str] = &["https://www.googleapis.com/auth/calendar.events.readonly"];

pub struct GoogleCalendarProvider {
    http_client: reqwest::blocking::Client,
}

impl GoogleCalendarProvider {
    pub fn new() -> Self {
        Self {
            http_client: build_http_client().unwrap_or_else(|e| {
                log::warn!("[google-calendar] Failed to build HTTP client: {e}");
                reqwest::blocking::Client::new()
            }),
        }
    }
}

impl Default for GoogleCalendarProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CalendarProvider for GoogleCalendarProvider {
    fn name(&self) -> &str {
        "Google Calendar"
    }

    fn icon(&self) -> &str {
        "calendar-symbolic"
    }

    fn auth_status(&mut self) -> AuthStatus {
        crate::services::google::google_auth_status()
    }

    fn authenticate(&mut self) -> Result<AuthStatus, String> {
        crate::services::google::google_authenticate(&CALENDAR_SCOPE.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        Ok(AuthStatus::Authenticated)
    }

    fn is_authenticated(&self) -> bool {
        crate::services::google::google_is_authenticated()
    }

    fn required_scopes(&self) -> &[&str] {
        CALENDAR_SCOPE
    }

    fn events(&mut self, start: &str, end: &str) -> Result<Vec<CalendarEvent>, String> {
        let mut reg = GoogleAuthRegistry::load()?;
        let token = reg.ensure_token(CALENDAR_SCOPE)?;

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime",
            start, end
        );

        #[derive(Deserialize)]
        struct Response {
            items: Option<Vec<Item>>,
        }

        #[derive(Deserialize)]
        struct Item {
            id: String,
            summary: Option<String>,
            start: Option<StartEnd>,
            end: Option<StartEnd>,
            location: Option<String>,
            #[serde(rename = "colorId")]
            color_id: Option<String>,
        }

        #[derive(Deserialize)]
        struct StartEnd {
            #[serde(rename = "dateTime")]
            date_time: Option<String>,
            date: Option<String>,
        }

        let resp: Response = api_get(&self.http_client, &url, &token)?;

        let events = resp
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                let summary = item.summary?;
                if summary.is_empty() {
                    return None;
                }

                let (start_str, all_day) = if let Some(dt) = item.start.as_ref().and_then(|s| s.date_time.clone()) {
                    (dt, false)
                } else if let Some(date) = item.start.as_ref().and_then(|s| s.date.clone()) {
                    (date, true)
                } else {
                    return None;
                };

                let end_str = item.end.as_ref().and_then(|e| {
                    e.date_time.clone().or(e.date.clone())
                }).unwrap_or_default();

                Some(CalendarEvent {
                    id: item.id,
                    summary,
                    start: start_str,
                    end: end_str,
                    all_day,
                    location: item.location,
                    color_id: item.color_id,
                })
            })
            .collect();

        Ok(events)
    }
}
