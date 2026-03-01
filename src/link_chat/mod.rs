mod discord;
mod line_api;
mod routing;
mod types;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use config::AppConfig;
use serenity::all::Http;
use serenity::prelude::RwLock;
use std::sync::Arc;

use discord::{send_content_to_discord, send_text_to_discord};
use line_api::{
    build_sticker_preview_url, get_group_member_profile, get_group_name, get_room_member_profile,
    get_room_name, get_user_profile, verify_line_signature,
};
use routing::{TemplateContext, render_message_text, render_webhook_name, resolve_target};
use types::{LineMessage, LineSource, LineWebhookPayload, LinkChatState};

pub fn spawn_line_to_discord_bridge(config: Arc<RwLock<AppConfig>>, http: Arc<Http>) {
    tokio::spawn(async move {
        let bind_addr = {
            let cfg = config.read().await;
            cfg.link_chat.bind.clone()
        };

        let state = LinkChatState::with_defaults(config, http);

        let app = Router::new()
            .route("/line/webhook", post(line_webhook_handler))
            .with_state(state);

        let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("[link-chat] bind failed on {}: {:?}", bind_addr, err);
                return;
            }
        };

        println!("[link-chat] listening on {}", bind_addr);

        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("[link-chat] server error: {:?}", err);
        }
    });
}

async fn line_webhook_handler(
    State(state): State<LinkChatState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    let link_chat_cfg = {
        let cfg = state.config.read().await;
        cfg.link_chat.clone()
    };

    if !link_chat_cfg.enabled {
        return Ok(StatusCode::NO_CONTENT);
    }

    if link_chat_cfg.line_channel_secret.is_empty()
        || link_chat_cfg.line_channel_access_token.is_empty()
    {
        eprintln!(
            "[link-chat] invalid config: missing line_channel_secret / line_channel_access_token"
        );
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let signature = headers
        .get("x-line-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !verify_line_signature(&link_chat_cfg.line_channel_secret, &body, signature) {
        eprintln!("[link-chat] signature verification failed");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let payload: LineWebhookPayload =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    for event in payload.events {
        let source_id = event.source.source_id().to_string();
        let mut group_name = match &event.source {
            LineSource::User { .. } => "DM".to_string(),
            LineSource::Group { .. } => "Unknown Group".to_string(),
            LineSource::Room { .. } => "Unknown Room".to_string(),
        };
        let mut sender_name = "Unknown Sender".to_string();
        let mut profile_avatar_url: Option<String> = None;

        if let Some(group_id) = event.source.group_id() {
            if let Some(name) = state.get_cached_group_name(group_id).await {
                group_name = name;
            } else if let Ok(name) = get_group_name(
                &state.reqwest,
                &link_chat_cfg.line_channel_access_token,
                group_id,
            )
            .await
            {
                if !name.is_empty() {
                    state
                        .set_cached_group_name(group_id.to_string(), name.clone())
                        .await;
                    group_name = name;
                }
            }
        } else if let Some(room_id) = event.source.room_id() {
            if let Some(name) = state.get_cached_room_name(room_id).await {
                group_name = name;
            } else if let Ok(name) = get_room_name(
                &state.reqwest,
                &link_chat_cfg.line_channel_access_token,
                room_id,
            )
            .await
            {
                if !name.is_empty() {
                    state
                        .set_cached_room_name(room_id.to_string(), name.clone())
                        .await;
                    group_name = name;
                }
            }
        }

        if let Some(user_id) = event.source.sender_user_id() {
            let profile_cache_key = if let Some(group_id) = event.source.group_id() {
                format!("group:{}:{}", group_id, user_id)
            } else if let Some(room_id) = event.source.room_id() {
                format!("room:{}:{}", room_id, user_id)
            } else {
                format!("user:{}", user_id)
            };

            let profile_result =
                if let Some(cached) = state.get_cached_profile(&profile_cache_key).await {
                    Ok(cached)
                } else {
                    let fetched = if let Some(group_id) = event.source.group_id() {
                        match get_group_member_profile(
                            &state.reqwest,
                            &link_chat_cfg.line_channel_access_token,
                            group_id,
                            user_id,
                        )
                        .await
                        {
                            Ok(profile) => Ok(profile),
                            Err(_) => {
                                get_user_profile(
                                    &state.reqwest,
                                    &link_chat_cfg.line_channel_access_token,
                                    user_id,
                                )
                                .await
                            }
                        }
                    } else if let Some(room_id) = event.source.room_id() {
                        match get_room_member_profile(
                            &state.reqwest,
                            &link_chat_cfg.line_channel_access_token,
                            room_id,
                            user_id,
                        )
                        .await
                        {
                            Ok(profile) => Ok(profile),
                            Err(_) => {
                                get_user_profile(
                                    &state.reqwest,
                                    &link_chat_cfg.line_channel_access_token,
                                    user_id,
                                )
                                .await
                            }
                        }
                    } else {
                        get_user_profile(
                            &state.reqwest,
                            &link_chat_cfg.line_channel_access_token,
                            user_id,
                        )
                        .await
                    };

                    if let Ok(profile) = &fetched {
                        state
                            .set_cached_profile(profile_cache_key, profile.clone())
                            .await;
                    }

                    fetched
                };

            if let Ok(profile) = profile_result {
                if !profile.display_name.is_empty() {
                    sender_name = profile.display_name;
                }
                if !profile.picture_url.is_empty() {
                    profile_avatar_url = Some(profile.picture_url);
                }
            } else {
                sender_name = "Unknown User".to_string();
            }
        } else {
            sender_name = "Unknown User".to_string();
        }

        println!(
            "[link-chat] received event: type={} source=<{}> sender=<{}> message={:?}",
            event.event_type, source_id, sender_name, event.message
        );

        if event.event_type != "message" || source_id.is_empty() {
            continue;
        }

        let Some(message) = event.message else {
            continue;
        };

        let Some(target) = resolve_target(&link_chat_cfg, &source_id) else {
            continue;
        };

        match message {
            LineMessage::Text { text } => {
                let event_time = if event.timestamp > 0 {
                    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(event.timestamp)
                        .unwrap_or_else(chrono::Utc::now)
                } else {
                    chrono::Utc::now()
                };
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: &text,
                    timestamp: event_time,
                };

                let content = render_message_text(&target, &context);
                let webhook_name = render_webhook_name(&target, &context);
                send_text_to_discord(
                    &state,
                    &target,
                    &content,
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
            LineMessage::Image { message_id } => {
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: "",
                    timestamp: chrono::Utc::now(),
                };
                let webhook_name = render_webhook_name(&target, &context);
                send_content_to_discord(
                    &state,
                    &target,
                    &link_chat_cfg.line_channel_access_token,
                    &message_id,
                    String::new(),
                    format!("line_image_{}.jpg", message_id),
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
            LineMessage::Video { message_id } => {
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: "",
                    timestamp: chrono::Utc::now(),
                };
                let webhook_name = render_webhook_name(&target, &context);
                send_content_to_discord(
                    &state,
                    &target,
                    &link_chat_cfg.line_channel_access_token,
                    &message_id,
                    String::new(),
                    format!("line_video_{}.mp4", message_id),
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
            LineMessage::Audio { message_id } => {
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: "",
                    timestamp: chrono::Utc::now(),
                };
                let webhook_name = render_webhook_name(&target, &context);
                send_content_to_discord(
                    &state,
                    &target,
                    &link_chat_cfg.line_channel_access_token,
                    &message_id,
                    String::new(),
                    format!("line_audio_{}.m4a", message_id),
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
            LineMessage::File {
                message_id,
                file_name,
            } => {
                let fallback_name = if file_name.is_empty() {
                    format!("line_file_{}.bin", message_id)
                } else {
                    file_name
                };
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: "",
                    timestamp: chrono::Utc::now(),
                };
                let webhook_name = render_webhook_name(&target, &context);
                send_content_to_discord(
                    &state,
                    &target,
                    &link_chat_cfg.line_channel_access_token,
                    &message_id,
                    String::new(),
                    fallback_name,
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
            LineMessage::Location {
                title,
                address,
                latitude,
                longitude,
            } => {
                let text = format!("[位置] {} {} ({}, {})", title, address, latitude, longitude);
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: &text,
                    timestamp: chrono::Utc::now(),
                };
                let webhook_name = render_webhook_name(&target, &context);
                send_text_to_discord(
                    &state,
                    &target,
                    &text,
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
            LineMessage::Sticker {
                sticker_id,
                sticker_resource_type,
            } => {
                let preview = build_sticker_preview_url(&sticker_id, &sticker_resource_type);
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: &preview,
                    timestamp: chrono::Utc::now(),
                };
                let webhook_name = render_webhook_name(&target, &context);
                send_text_to_discord(
                    &state,
                    &target,
                    &preview,
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
            LineMessage::Unknown => {
                let text = "[未知訊息型別]";
                let context = TemplateContext {
                    name: &sender_name,
                    avatar: profile_avatar_url.as_deref().unwrap_or(""),
                    user_id: event.source.sender_user_id().unwrap_or(""),
                    group_name: &group_name,
                    group_id: &source_id,
                    message: text,
                    timestamp: chrono::Utc::now(),
                };
                let webhook_name = render_webhook_name(&target, &context);
                send_text_to_discord(
                    &state,
                    &target,
                    text,
                    webhook_name.as_deref(),
                    profile_avatar_url.as_deref(),
                )
                .await?;
            }
        }
    }

    Ok(StatusCode::OK)
}
