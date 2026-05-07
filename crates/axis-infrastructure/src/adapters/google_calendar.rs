use async_trait::async_trait;
use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::calendar::{CalendarError, CalendarProvider};
use axis_domain::ports::cloud_auth::CloudAuthProvider;
use chrono::NaiveDateTime;
use log::{debug, info, warn};
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

pub struct GoogleCalendarProvider {
    auth_provider: Arc<dyn CloudAuthProvider>,
    http_client: reqwest::Client,
}

impl GoogleCalendarProvider {
    pub fn new(auth_provider: Arc<dyn CloudAuthProvider>) -> Arc<Self> {
        Arc::new(Self {
            auth_provider,
            http_client: reqwest::Client::builder()
                .tcp_keepalive(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        })
    }

    async fn fetch_events_from_calendar(
        &self,
        calendar_id: &str,
        start: &str,
        end: &str,
        token: &str,
    ) -> Result<Vec<CalendarEvent>, CalendarError> {
        let url = format!(
            "{}/calendars/{}/events",
            GOOGLE_CALENDAR_API_URL,
            urlencoding::encode(calendar_id)
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("timeMin", start),
                ("timeMax", end),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
            ])
            .send()
            .await
            .map_err(|e| CalendarError::ProviderError(format!("Network error: {}", e)))?;

        if !response.status().is_success() {
            return Err(CalendarError::ProviderError(format!(
                "API error: {}",
                response.status()
            )));
        }

        let event_list: GoogleEventList = response
            .json()
            .await
            .map_err(|e| CalendarError::ProviderError(format!("JSON error: {}", e)))?;

        Ok(event_list
            .items
            .into_iter()
            .map(|e| {
                let (start_time, all_day) = if let Some(dt) = e.start.date_time {
                    (parse_datetime(&dt), false)
                } else {
                    (
                        parse_date(e.start.date.as_deref().unwrap_or("1970-01-01")),
                        true,
                    )
                };

                let end_time = if let Some(dt) = e.end.date_time {
                    parse_datetime(&dt)
                } else {
                    parse_date(e.end.date.as_deref().unwrap_or("1970-01-01"))
                };

                CalendarEvent {
                    id: e.id,
                    summary: e.summary.unwrap_or_else(|| "(No title)".to_string()),
                    start: start_time,
                    end: end_time,
                    all_day,
                    color_id: e.color_id,
                }
            })
            .collect())
    }
}

#[async_trait]
impl CalendarProvider for GoogleCalendarProvider {
    async fn get_events(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<CalendarEvent>, CalendarError> {
        let scopes = vec!["https://www.googleapis.com/auth/calendar.readonly".to_string()];
        let token = self
            .auth_provider
            .get_token(&scopes)
            .await
            .map_err(|e| CalendarError::ProviderError(format!("Auth error: {}", e)))?;

        // 1. Get list of primary calendars
        let resp = self
            .http_client
            .get(format!("{}/users/me/calendarList", GOOGLE_CALENDAR_API_URL))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| CalendarError::ProviderError(e.to_string()))?;

        let cal_list: GoogleCalendarList = resp
            .json()
            .await
            .map_err(|e| CalendarError::ProviderError(e.to_string()))?;

        let mut fetch_tasks = Vec::new();
        for cal in cal_list.items {
            let calendar_id = cal.id.clone();
            let start = start.to_string();
            let end = end.to_string();
            let token = token.clone();

            fetch_tasks.push(async move {
                self.fetch_events_from_calendar(&calendar_id, &start, &end, &token)
                    .await
            });
        }

        let results = futures_util::future::join_all(fetch_tasks).await;
        let mut all_events = Vec::new();

        for (idx, res) in results.into_iter().enumerate() {
            match res {
                Ok(events) => {
                    debug!(
                        "[google-calendar] Fetched {} events from calendar {}",
                        events.len(),
                        idx
                    );
                    all_events.extend(events);
                }
                Err(e) => warn!(
                    "[google-calendar] Failed to fetch events from calendar {}: {}",
                    idx, e
                ),
            }
        }

        info!(
            "[google-calendar] Total events fetched: {} (Parallel)",
            all_events.len()
        );
        all_events.sort_by(|a, b| a.start.cmp(&b.start));
        Ok(all_events)
    }
}

fn parse_datetime(s: &str) -> NaiveDateTime {
    // Try RFC3339 first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.naive_utc();
    }
    // Try common ISO formats
    let cleaned = s.trim_end_matches('Z').split('+').next().unwrap_or(s);
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%dT%H:%M:%S") {
        return dt;
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%dT%H:%M:%S%.f") {
        return dt;
    }
    chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc()
}

fn parse_date(s: &str) -> NaiveDateTime {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc())
}
