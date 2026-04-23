use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::calendar::{CalendarProvider, CalendarError};
use axis_domain::ports::cloud_auth::CloudAuthProvider;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

const GOOGLE_CALENDAR_API_URL: &str = "https://www.googleapis.com/calendar/v3";

#[derive(Deserialize)]
struct GoogleCalendarList {
    items: Vec<GoogleCalendarItem>,
}

#[derive(Deserialize)]
struct GoogleCalendarItem {
    id: String,
}

#[derive(Deserialize)]
struct GoogleEventList {
    items: Vec<GoogleEvent>,
}

#[derive(Deserialize)]
struct GoogleEvent {
    id: String,
    summary: Option<String>,
    start: GoogleDateTime,
    end: GoogleEventDateTime,
    #[serde(rename = "colorId")]
    color_id: Option<String>,
}

#[derive(Deserialize)]
struct GoogleDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Deserialize)]
struct GoogleEventDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

pub struct GoogleCalendarAdapter {
    auth_provider: Arc<dyn CloudAuthProvider>,
}

impl GoogleCalendarAdapter {
    pub fn new(auth_provider: Arc<dyn CloudAuthProvider>) -> Self {
        Self { auth_provider }
    }

    async fn fetch_events_from_calendar(&self, calendar_id: &str, start: &str, end: &str, token: &str) -> Result<Vec<CalendarEvent>, CalendarError> {
        let client = reqwest::Client::new();
        let url = format!("{}/calendars/{}/events", GOOGLE_CALENDAR_API_URL, urlencoding::encode(calendar_id));
        
        let response = client.get(&url)
            .bearer_auth(token)
            .query(&[
                ("timeMin", start),
                ("timeMax", end),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
            ])
            .send().await
            .map_err(|e| CalendarError::ProviderError(format!("Network error: {}", e)))?;

        if !response.status().is_success() {
            return Err(CalendarError::ProviderError(format!("API error: {}", response.status())));
        }

        let event_list: GoogleEventList = response.json().await
            .map_err(|e| CalendarError::ProviderError(format!("JSON error: {}", e)))?;

        Ok(event_list.items.into_iter().map(|e| {
            let (start_time, all_day) = if let Some(dt) = e.start.date_time {
                (dt, false)
            } else {
                (e.start.date.unwrap_or_default(), true)
            };

            let end_time = e.end.date_time.or(e.end.date).unwrap_or_default();

            CalendarEvent {
                id: e.id,
                summary: e.summary.unwrap_or_else(|| "(No title)".to_string()),
                start: start_time,
                end: end_time,
                all_day,
                color_id: e.color_id,
            }
        }).collect())
    }
}

#[async_trait]
impl CalendarProvider for GoogleCalendarAdapter {
    async fn get_events(&self, start: &str, end: &str) -> Result<Vec<CalendarEvent>, CalendarError> {
        let scopes = vec!["https://www.googleapis.com/auth/calendar.readonly".to_string()];
        let token = self.auth_provider.get_token(&scopes).await
            .map_err(|e| CalendarError::ProviderError(format!("Auth error: {}", e)))?;

        let client = reqwest::Client::new();
        
        // 1. Get list of primary calendars
        let resp = client.get(format!("{}/users/me/calendarList", GOOGLE_CALENDAR_API_URL))
            .bearer_auth(&token)
            .send().await
            .map_err(|e| CalendarError::ProviderError(e.to_string()))?;
            
        let cal_list: GoogleCalendarList = resp.json().await
            .map_err(|e| CalendarError::ProviderError(e.to_string()))?;

        let mut all_events = Vec::new();
        for cal in cal_list.items {
            match self.fetch_events_from_calendar(&cal.id, start, end, &token).await {
                Ok(events) => {
                    log::debug!("[google-calendar] Fetched {} events from calendar {}", events.len(), cal.id);
                    all_events.extend(events);
                }
                Err(e) => log::warn!("[google-calendar] Failed to fetch events from {}: {}", cal.id, e),
            }
        }

        log::info!("[google-calendar] Total events fetched: {}", all_events.len());
        all_events.sort_by(|a, b| a.start.cmp(&b.start));
        Ok(all_events)
    }
}
