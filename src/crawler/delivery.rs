use config::CrawlerEntryConfig;
use reqwest::Client;
use serde_json::json;
use serenity::all::{ChannelId, Http};
use serenity::builder::CreateMessage;
use std::sync::Arc;

pub(crate) async fn send_to_discord(
    reqwest: &Client,
    http: &Arc<Http>,
    entry: &CrawlerEntryConfig,
    content: &str,
) -> Result<(), String> {
    if !entry.notifications.discord.webhook_url.is_empty() {
        let response = reqwest
            .post(&entry.notifications.discord.webhook_url)
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(|err| format!("webhook request failed: {err}"))?;

        response
            .error_for_status()
            .map_err(|err| format!("webhook status failed: {err}"))?;
        return Ok(());
    }

    if entry.notifications.discord.channel_id == 0 {
        return Err(
            "missing discord target: set discord_channel_id or discord_webhook_url".to_string(),
        );
    }

    ChannelId::new(entry.notifications.discord.channel_id)
        .send_message(http, CreateMessage::new().content(content))
        .await
        .map_err(|err| format!("send message failed: {err}"))?;

    Ok(())
}
