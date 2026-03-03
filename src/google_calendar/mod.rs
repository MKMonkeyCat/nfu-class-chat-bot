mod api;
mod auth;
mod notify;
mod store;
mod types;

use config::CrawlerConfig;
use reqwest::Client;
use sea_orm::DatabaseConnection;
use serenity::all::Http;
use serenity::prelude::RwLock;
use std::sync::Arc;
use std::time::Duration;

use api::fetch_upcoming_events;
use auth::get_or_refresh_access_token;
use notify::process_event;
use types::CachedGoogleToken;

#[allow(dead_code)]
pub fn spawn_google_calendar_sync(
    config: Arc<RwLock<CrawlerConfig>>,
    http: Arc<Http>,
    db: DatabaseConnection,
) {
    tokio::spawn(async move {
        let reqwest = Client::new();
        let mut token_cache: Option<CachedGoogleToken> = None;

        loop {
            let cfg = {
                let guard = config.read().await;
                guard.clone()
            };

            let gcal = cfg.google_calendar.clone();
            if !gcal.enabled {
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }

            let enabled_calendars = gcal
                .calendars
                .iter()
                .filter(|calendar| calendar.enabled && !calendar.calendar_id.trim().is_empty())
                .cloned()
                .collect::<Vec<_>>();

            if enabled_calendars.is_empty() {
                tokio::time::sleep(Duration::from_secs(gcal.poll_interval_seconds.max(30))).await;
                continue;
            }

            let token = match get_or_refresh_access_token(&reqwest, &gcal, &mut token_cache).await {
                Ok(token) => token,
                Err(err) => {
                    eprintln!("[calendar] get access token failed: {}", err);
                    tokio::time::sleep(Duration::from_secs(gcal.poll_interval_seconds.max(30)))
                        .await;
                    continue;
                }
            };

            for calendar in enabled_calendars {
                let events = match fetch_upcoming_events(&reqwest, &token, &gcal, &calendar).await {
                    Ok(events) => events,
                    Err(err) => {
                        eprintln!("[calendar] fetch '{}' failed: {}", calendar.name, err);
                        continue;
                    }
                };

                for event in events {
                    if let Err(err) = process_event(
                        &reqwest,
                        &http,
                        &db,
                        &cfg.notifications,
                        &gcal,
                        &calendar,
                        event,
                    )
                    .await
                    {
                        eprintln!(
                            "[calendar] process event failed (calendar={}): {}",
                            calendar.name, err
                        );
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(gcal.poll_interval_seconds.max(30))).await;
        }
    });
}
