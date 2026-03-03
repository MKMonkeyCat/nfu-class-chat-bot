use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct CachedGoogleToken {
    pub(crate) access_token: String,
    pub(crate) expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleServiceAccount {
    pub(crate) client_email: String,
    pub(crate) private_key: String,
    pub(crate) token_uri: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoogleJwtClaims {
    pub(crate) iss: String,
    pub(crate) scope: String,
    pub(crate) aud: String,
    pub(crate) iat: i64,
    pub(crate) exp: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct GoogleTokenRequest<'a> {
    pub(crate) grant_type: &'a str,
    pub(crate) assertion: &'a str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleTokenResponse {
    pub(crate) access_token: String,
    pub(crate) expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleCalendarEventsResponse {
    #[serde(default)]
    pub(crate) items: Vec<GoogleCalendarEvent>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleCalendarEvent {
    pub(crate) id: Option<String>,
    pub(crate) summary: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) location: Option<String>,
    #[serde(rename = "htmlLink")]
    pub(crate) html_link: Option<String>,
    pub(crate) start: GoogleCalendarEventDate,
    pub(crate) end: Option<GoogleCalendarEventDate>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleCalendarEventDate {
    #[serde(rename = "dateTime")]
    pub(crate) date_time: Option<String>,
    pub(crate) date: Option<String>,
}

impl GoogleCalendarEventDate {
    pub(crate) fn key(&self) -> String {
        if let Some(value) = &self.date_time {
            return value.clone();
        }
        self.date.clone().unwrap_or_default()
    }

    pub(crate) fn display_text(&self) -> Option<String> {
        if let Some(value) = &self.date_time {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
                return Some(
                    dt.with_timezone(&Utc)
                        .format("%Y-%m-%d %H:%M UTC")
                        .to_string(),
                );
            }
            return Some(value.clone());
        }

        self.date.clone()
    }
}
