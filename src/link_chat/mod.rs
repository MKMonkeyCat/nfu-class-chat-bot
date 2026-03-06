mod discord;
mod line_api;
mod routing;
mod types;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::post;
use config::AppConfig;
use serenity::all::Http;
use serenity::prelude::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use discord::{send_content_to_discord, send_text_to_discord};
use line_api::{
    build_sticker_preview_url, get_group_member_profile, get_group_name, get_room_member_profile,
    get_room_name, get_user_profile, verify_line_signature,
};
use routing::{TemplateContext, render_message_text, render_webhook_name, resolve_target};
use types::{LineMessage, LineSource, LineWebhookPayload, LinkChatState};

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);

fn make_template_context<'a>(
    sender_name: &'a str,
    avatar: &'a str,
    user_id: &'a str,
    group_name: &'a str,
    source_id: &'a str,
    message: &'a str,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> TemplateContext<'a> {
    TemplateContext {
        name: sender_name,
        avatar,
        user_id,
        group_name,
        group_id: source_id,
        message,
        timestamp,
    }
}

fn next_request_id() -> String {
    let seq = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("linkchat-{}", seq)
}

async fn request_log_middleware(mut req: Request<axum::body::Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or_else(next_request_id);

    if let Ok(v) = HeaderValue::from_str(&request_id) {
        req.headers_mut().insert("x-request-id", v);
    }

    let mut response = next.run(req).await;
    let status = response.status();
    let elapsed_ms = start.elapsed().as_millis();

    if let Ok(v) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", v);
    }

    println!(
        "[link-chat][req] id={} method={} path={} status={} latency_ms={}",
        request_id,
        method,
        path,
        status.as_u16(),
        elapsed_ms
    );

    response
}

pub fn spawn_line_to_discord_bridge(config: Arc<RwLock<AppConfig>>, http: Arc<Http>) {
    tokio::spawn(async move {
        let bind_addr = {
            let cfg = config.read().await;
            cfg.link_chat.bind.clone()
        };

        let state = LinkChatState::with_defaults(config, http);

        let app = Router::new()
            .route("/line/webhook", post(line_webhook_handler))
            .layer(middleware::from_fn(request_log_middleware))
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

    println!(
        "[link-chat] received webhook: content_length={} bytes payload={:?}",
        body.len(),
        body
    );

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

        let sender_user_id = event.source.sender_user_id().unwrap_or("");
        let avatar_for_template = profile_avatar_url.as_deref().unwrap_or("");

        match message {
            LineMessage::Text { text } => {
                let event_time = if event.timestamp > 0 {
                    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(event.timestamp)
                        .unwrap_or_else(chrono::Utc::now)
                } else {
                    chrono::Utc::now()
                };
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    &text,
                    event_time,
                );

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
            LineMessage::Image {
                message_id,
                image_set,
            } => {
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    "",
                    chrono::Utc::now(),
                );
                let webhook_name = render_webhook_name(&target, &context);

                if let Some(image_set) = image_set {
                    if !image_set.id.is_empty()
                        && image_set.total > 1
                        && image_set.index > 0
                        && !message_id.is_empty()
                    {
                        let maybe_message_ids = state
                            .add_image_set_message(
                                &source_id,
                                &image_set.id,
                                image_set.index,
                                image_set.total,
                                message_id,
                            )
                            .await;

                        if let Some(message_ids) = maybe_message_ids {
                            for (idx, image_message_id) in message_ids.iter().enumerate() {
                                let caption = if idx == 0 {
                                    format!("[LINE 圖片組，共 {} 張]", image_set.total)
                                } else {
                                    String::new()
                                };

                                send_content_to_discord(
                                    &state,
                                    &target,
                                    &link_chat_cfg.line_channel_access_token,
                                    image_message_id,
                                    caption,
                                    format!("line_imageset_{}_{}.jpg", image_set.id, idx + 1),
                                    webhook_name.as_deref(),
                                    profile_avatar_url.as_deref(),
                                )
                                .await?;
                            }
                        }

                        continue;
                    }
                }

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
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    "",
                    chrono::Utc::now(),
                );
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
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    "",
                    chrono::Utc::now(),
                );
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
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    "",
                    chrono::Utc::now(),
                );
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
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    &text,
                    chrono::Utc::now(),
                );
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
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    &preview,
                    chrono::Utc::now(),
                );
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
                let context = make_template_context(
                    &sender_name,
                    avatar_for_template,
                    sender_user_id,
                    &group_name,
                    &source_id,
                    text,
                    chrono::Utc::now(),
                );
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
