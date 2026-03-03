use config::{GoogleCalendarConfig, GoogleCalendarEntryConfig, NotificationConfig};
use reqwest::Client;
use sea_orm::DatabaseConnection;
use serenity::all::Http;
use std::collections::HashMap;
use std::sync::Arc;

use crate::discord::{DiscordDeliveryTarget, DiscordEmbedField, DiscordEmbedPayload, send_embed};

use super::store::{is_event_seen, mark_event_seen};
use super::types::{GoogleCalendarEvent, GoogleCalendarEventDate};

pub(crate) async fn process_event(
    reqwest: &Client,
    http: &Arc<Http>,
    db: &DatabaseConnection,
    notifications: &HashMap<String, NotificationConfig>,
    gcal: &GoogleCalendarConfig,
    calendar: &GoogleCalendarEntryConfig,
    event: GoogleCalendarEvent,
) -> Result<(), String> {
    let event_id = event.id.unwrap_or_default().trim().to_string();
    let summary = event
        .summary
        .clone()
        .unwrap_or_else(|| "(無標題活動)".to_string());

    if event_id.is_empty() {
        return Ok(());
    }

    let start_key = event.start.key();
    if start_key.is_empty() {
        return Ok(());
    }

    if is_event_seen(db, &calendar.calendar_id, &event_id, &start_key).await? {
        return Ok(());
    }

    let targets = selected_notifications(calendar, gcal, notifications);
    if targets.is_empty() {
        return Ok(());
    }

    let title = format!("📅 {}", summary);
    let (start_text, end_text) = format_event_time_range(&event.start, &event.end);

    let mut fields = vec![DiscordEmbedField {
        name: "日曆".to_string(),
        value: calendar.name.clone(),
        inline: true,
    }];

    if !start_text.is_empty() {
        fields.push(DiscordEmbedField {
            name: "開始".to_string(),
            value: start_text,
            inline: true,
        });
    }

    if !end_text.is_empty() {
        fields.push(DiscordEmbedField {
            name: "結束".to_string(),
            value: end_text,
            inline: true,
        });
    }

    if let Some(location) = event.location.clone() {
        if !location.trim().is_empty() {
            fields.push(DiscordEmbedField {
                name: "地點".to_string(),
                value: location,
                inline: false,
            });
        }
    }

    let description = event.description.unwrap_or_default();
    let description = if description.trim().is_empty() {
        "Google Calendar 活動通知".to_string()
    } else {
        description.chars().take(400).collect::<String>()
    };

    let event_url = event.html_link.unwrap_or_default();
    let mut delivered = false;
    for notification in targets {
        let embed = DiscordEmbedPayload {
            title: title.clone(),
            description: description.clone(),
            url: event_url.clone(),
            color: notification.discord.embed_color,
            fields: fields.clone(),
            footer: calendar.calendar_id.clone(),
        };

        let target = DiscordDeliveryTarget {
            channel_id: notification.discord.channel_id,
            webhook_url: notification.discord.webhook_url.clone(),
            webhook_avatar_url: notification.discord.webhook_avatar_url.clone(),
        };

        if let Err(err) =
            send_embed(reqwest, http, &target, &embed, None, Some("Calendar"), None).await
        {
            eprintln!("[calendar] send embed failed ({}): {}", calendar.name, err);
        } else {
            delivered = true;
        }
    }

    if delivered {
        mark_event_seen(db, &calendar.calendar_id, &event_id, &start_key).await?;
    }

    Ok(())
}

fn selected_notifications<'a>(
    calendar: &GoogleCalendarEntryConfig,
    gcal: &GoogleCalendarConfig,
    notifications: &'a HashMap<String, NotificationConfig>,
) -> Vec<&'a NotificationConfig> {
    let target_ids = if calendar.notify_targets.is_empty() {
        &gcal.notify_targets
    } else {
        &calendar.notify_targets
    };

    if target_ids.is_empty() {
        return notifications.values().filter(|n| n.enabled).collect();
    }

    target_ids
        .iter()
        .filter_map(|id| notifications.get(id))
        .filter(|notification| notification.enabled)
        .collect()
}

fn format_event_time_range(
    start: &GoogleCalendarEventDate,
    end: &Option<GoogleCalendarEventDate>,
) -> (String, String) {
    let start_text = start
        .display_text()
        .or_else(|| start.date.clone())
        .unwrap_or_default();

    let end_text = end
        .as_ref()
        .and_then(|value| value.display_text().or_else(|| value.date.clone()))
        .unwrap_or_default();

    (start_text, end_text)
}
