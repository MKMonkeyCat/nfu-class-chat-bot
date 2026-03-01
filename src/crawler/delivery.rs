use crate::discord::{DiscordDeliveryTarget, send_embed};
use config::NotificationConfig;
use reqwest::Client;
use serenity::all::Http;
use std::sync::Arc;

use super::message::CrawlerDiscordMessage;

pub(crate) async fn send_to_discord(
    reqwest: &Client,
    http: &Arc<Http>,
    notification: &NotificationConfig,
    message: &CrawlerDiscordMessage,
) -> Result<(), String> {
    let target = DiscordDeliveryTarget {
        channel_id: notification.discord.channel_id,
        webhook_url: notification.discord.webhook_url.clone(),
        webhook_avatar_url: notification.discord.webhook_avatar_url.clone(),
    };

    send_embed(
        reqwest,
        http,
        &target,
        &message.embed,
        message.content.as_deref(),
        message.webhook_username.as_deref(),
        None,
    )
    .await
}
