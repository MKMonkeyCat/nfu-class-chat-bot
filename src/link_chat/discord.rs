use crate::discord::{
    DiscordDeliveryTarget, bad_gateway_from_error, is_missing_target_error, send_file, send_text,
};
use reqwest::StatusCode;

use super::line_api::{fetch_line_content, infer_filename};
use super::types::{DeliveryTarget, LinkChatState};

pub(super) async fn send_text_to_discord(
    state: &LinkChatState,
    target: &DeliveryTarget,
    content: &str,
    webhook_username: Option<&str>,
    webhook_avatar_override: Option<&str>,
) -> Result<(), StatusCode> {
    let delivery_target = DiscordDeliveryTarget {
        channel_id: target.discord_channel_id,
        webhook_url: target.discord_webhook_url.clone(),
        webhook_avatar_url: target.discord_webhook_avatar_url.clone(),
    };

    send_text(
        &state.reqwest,
        &state.http,
        &delivery_target,
        content,
        webhook_username,
        webhook_avatar_override,
    )
    .await
    .map_err(|error| {
        if is_missing_target_error(&error) {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            bad_gateway_from_error(&error)
        }
    })?;

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

    let delivery_target = DiscordDeliveryTarget {
        channel_id: target.discord_channel_id,
        webhook_url: target.discord_webhook_url.clone(),
        webhook_avatar_url: target.discord_webhook_avatar_url.clone(),
    };

    send_file(
        &state.reqwest,
        &state.http,
        &delivery_target,
        bytes,
        filename,
        &caption,
        webhook_username,
        webhook_avatar_override,
    )
    .await
    .map_err(|error| {
        if is_missing_target_error(&error) {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            bad_gateway_from_error(&error)
        }
    })?;

    Ok(())
}
