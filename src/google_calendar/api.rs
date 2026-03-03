use chrono::{Duration as ChronoDuration, Utc};
use config::{GoogleCalendarConfig, GoogleCalendarEntryConfig};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Client;

use super::types::{GoogleCalendarEvent, GoogleCalendarEventsResponse};

pub(crate) async fn fetch_upcoming_events(
    reqwest: &Client,
    token: &str,
    gcal: &GoogleCalendarConfig,
    calendar: &GoogleCalendarEntryConfig,
) -> Result<Vec<GoogleCalendarEvent>, String> {
    let now = Utc::now();
    let max_time = now + ChronoDuration::hours(gcal.lookahead_hours.max(1) as i64);
    let encoded_calendar = utf8_percent_encode(&calendar.calendar_id, NON_ALPHANUMERIC).to_string();

    let endpoint = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events",
        encoded_calendar
    );

    let response = reqwest
        .get(endpoint)
        .bearer_auth(token)
        .query(&[
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
            ("timeMin", &now.to_rfc3339()),
            ("timeMax", &max_time.to_rfc3339()),
            ("maxResults", &gcal.max_results.max(1).to_string()),
        ])
        .send()
        .await
        .map_err(|err| format!("request events failed: {err}"))?
        .error_for_status()
        .map_err(|err| format!("events status failed: {err}"))?;

    let payload = response
        .json::<GoogleCalendarEventsResponse>()
        .await
        .map_err(|err| format!("parse events response failed: {err}"))?;

    Ok(payload.items)
}
