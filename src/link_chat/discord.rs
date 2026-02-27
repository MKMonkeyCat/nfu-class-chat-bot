use reqwest::StatusCode;
use reqwest::multipart::{Form, Part};
use serde_json::json;
use serenity::all::ChannelId;
use serenity::builder::{CreateAttachment, CreateMessage};

use super::line_api::{fetch_line_content, infer_filename};
use super::types::{DeliveryTarget, LinkChatState};

pub(super) async fn send_text_to_discord(
    state: &LinkChatState,
    target: &DeliveryTarget,
    content: &str,
    webhook_username: Option<&str>,
    webhook_avatar_override: Option<&str>,
) -> Result<(), StatusCode> {
    if !target.discord_webhook_url.is_empty() {
        let response = state
            .reqwest
            .post(&target.discord_webhook_url)
            .json(&json!({
                "content": content,
                "username": webhook_username,
                "avatar_url": if let Some(avatar) = webhook_avatar_override {
                    Some(avatar.to_string())
                } else if target.discord_webhook_avatar_url.is_empty() {
                    None::<String>
                } else {
                    Some(target.discord_webhook_avatar_url.clone())
                },
            }))
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        return Ok(());
    }

    if target.discord_channel_id == 0 {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    ChannelId::new(target.discord_channel_id)
        .say(&state.http, content)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(())
}

pub(super) async fn send_content_to_discord(
    state: &LinkChatState,
    target: &DeliveryTarget,
    access_token: &str,
    message_id: &str,
    caption: String,
    default_filename: String,
    webhook_username: Option<&str>,
    webhook_avatar_override: Option<&str>,
) -> Result<(), StatusCode> {
    if message_id.is_empty() {
        return send_text_to_discord(
            state,
            target,
            &format!("{}（缺少 message id，無法下載內容）", caption),
            webhook_username,
            webhook_avatar_override,
        )
        .await;
    }

    let (bytes, content_type) = fetch_line_content(&state.reqwest, access_token, message_id)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let filename = infer_filename(default_filename, &content_type);

    if !target.discord_webhook_url.is_empty() {
        let payload_json = json!({
            "content": caption,
            "username": webhook_username,
            "avatar_url": if let Some(avatar) = webhook_avatar_override {
                Some(avatar.to_string())
            } else if target.discord_webhook_avatar_url.is_empty() {
                None::<String>
            } else {
                Some(target.discord_webhook_avatar_url.clone())
            },
        })
        .to_string();

        let file_part = Part::bytes(bytes).file_name(filename);
        let form = Form::new()
            .text("payload_json", payload_json)
            .part("files[0]", file_part);

        let response = state
            .reqwest
            .post(&target.discord_webhook_url)
            .multipart(form)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }

        return Ok(());
    }

    if target.discord_channel_id == 0 {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let attachment = CreateAttachment::bytes(bytes, filename);
    ChannelId::new(target.discord_channel_id)
        .send_files(
            &state.http,
            vec![attachment],
            CreateMessage::new().content(caption),
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(())
}
