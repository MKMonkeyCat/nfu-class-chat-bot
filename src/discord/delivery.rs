use reqwest::Client;
use reqwest::multipart::{Form, Part};
use serde_json::json;
use serenity::all::{ChannelId, Http};
use serenity::builder::{CreateAttachment, CreateEmbed, CreateEmbedFooter, CreateMessage};
use std::sync::Arc;

use super::types::{DiscordDeliveryTarget, DiscordEmbedPayload};
use super::utils::MISSING_TARGET_ERROR;

pub(crate) async fn send_text(
    reqwest: &Client,
    http: &Arc<Http>,
    target: &DiscordDeliveryTarget,
    content: &str,
    webhook_username: Option<&str>,
    webhook_avatar_override: Option<&str>,
) -> Result<(), String> {
    if !target.webhook_url.is_empty() {
        let response = reqwest
            .post(&target.webhook_url)
            .json(&json!({
                "content": content,
                "username": webhook_username,
                "avatar_url": if let Some(avatar) = webhook_avatar_override {
                    Some(avatar.to_string())
                } else if target.webhook_avatar_url.is_empty() {
                    None::<String>
                } else {
                    Some(target.webhook_avatar_url.clone())
                },
            }))
            .send()
            .await
            .map_err(|err| format!("webhook request failed: {err}"))?;

        response
            .error_for_status()
            .map_err(|err| format!("webhook status failed: {err}"))?;
        return Ok(());
    }

    if target.channel_id == 0 {
        return Err(MISSING_TARGET_ERROR.into());
    }

    ChannelId::new(target.channel_id)
        .send_message(http, CreateMessage::new().content(content))
        .await
        .map_err(|err| format!("send message failed: {err}"))?;

    Ok(())
}

pub(crate) async fn send_file(
    reqwest: &Client,
    http: &Arc<Http>,
    target: &DiscordDeliveryTarget,
    bytes: Vec<u8>,
    filename: String,
    caption: &str,
    webhook_username: Option<&str>,
    webhook_avatar_override: Option<&str>,
) -> Result<(), String> {
    if !target.webhook_url.is_empty() {
        let payload_json = json!({
            "content": caption,
            "username": webhook_username,
            "avatar_url": if let Some(avatar) = webhook_avatar_override {
                Some(avatar.to_string())
            } else if target.webhook_avatar_url.is_empty() {
                None::<String>
            } else {
                Some(target.webhook_avatar_url.clone())
            },
        })
        .to_string();

        let file_part = Part::bytes(bytes).file_name(filename);
        let form = Form::new()
            .text("payload_json", payload_json)
            .part("files[0]", file_part);

        let response = reqwest
            .post(&target.webhook_url)
            .multipart(form)
            .send()
            .await
            .map_err(|err| format!("webhook multipart request failed: {err}"))?;

        response
            .error_for_status()
            .map_err(|err| format!("webhook status failed: {err}"))?;
        return Ok(());
    }

    if target.channel_id == 0 {
        return Err(MISSING_TARGET_ERROR.into());
    }

    let attachment = CreateAttachment::bytes(bytes, filename);
    ChannelId::new(target.channel_id)
        .send_files(
            http,
            vec![attachment],
            CreateMessage::new().content(caption),
        )
        .await
        .map_err(|err| format!("send file failed: {err}"))?;

    Ok(())
}

pub(crate) async fn send_embed(
    reqwest: &Client,
    http: &Arc<Http>,
    target: &DiscordDeliveryTarget,
    embed: &DiscordEmbedPayload,
    content: Option<&str>,
    webhook_username: Option<&str>,
    webhook_avatar_override: Option<&str>,
) -> Result<(), String> {
    if !target.webhook_url.is_empty() {
        let response = reqwest
            .post(&target.webhook_url)
            .json(&json!({
                "content": content,
                "username": webhook_username,
                "avatar_url": if let Some(avatar) = webhook_avatar_override {
                    Some(avatar.to_string())
                } else if target.webhook_avatar_url.is_empty() {
                    None::<String>
                } else {
                    Some(target.webhook_avatar_url.clone())
                },
                "embeds": [
                    {
                        "title": embed.title,
                        "description": embed.description,
                        "url": if embed.url.trim().is_empty() { None::<String> } else { Some(embed.url.clone()) },
                        "color": embed.color,
                        "footer": if embed.footer.trim().is_empty() {
                            None::<serde_json::Value>
                        } else {
                            Some(json!({ "text": embed.footer }))
                        },
                        "fields": embed.fields.iter().map(|field| {
                            json!({
                                "name": field.name,
                                "value": field.value,
                                "inline": field.inline,
                            })
                        }).collect::<Vec<_>>(),
                    }
                ]
            }))
            .send()
            .await
            .map_err(|err| format!("webhook embed request failed: {err}"))?;

        response
            .error_for_status()
            .map_err(|err| format!("webhook status failed: {err}"))?;
        return Ok(());
    }

    if target.channel_id == 0 {
        return Err(MISSING_TARGET_ERROR.into());
    }

    let mut discord_embed = CreateEmbed::new()
        .title(embed.title.clone())
        .description(embed.description.clone())
        .color(embed.color);

    if !embed.url.trim().is_empty() {
        discord_embed = discord_embed.url(embed.url.clone());
    }

    if !embed.footer.trim().is_empty() {
        discord_embed = discord_embed.footer(CreateEmbedFooter::new(embed.footer.clone()));
    }

    for field in &embed.fields {
        discord_embed = discord_embed.field(field.name.clone(), field.value.clone(), field.inline);
    }

    let mut message = CreateMessage::new().embed(discord_embed);
    if let Some(text) = content {
        if !text.trim().is_empty() {
            message = message.content(text);
        }
    }

    ChannelId::new(target.channel_id)
        .send_message(http, message)
        .await
        .map_err(|err| format!("send embed failed: {err}"))?;

    Ok(())
}
