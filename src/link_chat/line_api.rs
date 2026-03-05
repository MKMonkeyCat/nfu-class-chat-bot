use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;

use crate::link_chat::types::MemberProfile;

type HmacSha256 = Hmac<Sha256>;

pub(super) fn verify_line_signature(secret: &str, body: &[u8], expected_signature: &str) -> bool {
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };

    mac.update(body);
    let result = mac.finalize().into_bytes();
    let signature = base64::engine::general_purpose::STANDARD.encode(result);
    signature == expected_signature
}

pub(super) async fn fetch_line_content(
    client: &Client,
    access_token: &str,
    message_id: &str,
) -> Result<(Vec<u8>, String), String> {
    let url = format!(
        "https://api-data.line.me/v2/bot/message/{}/content",
        message_id
    );

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| format!("request error: {:?}", err))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(format!("LINE API returned {}: {}", status, body));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("read bytes error: {:?}", err))?
        .to_vec();

    Ok((bytes, content_type))
}

pub(super) fn infer_filename(default_filename: String, content_type: &str) -> String {
    if default_filename.contains('.') {
        return default_filename;
    }

    let ext = match content_type.split(';').next().unwrap_or_default().trim() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "video/mp4" => "mp4",
        "audio/mpeg" => "mp3",
        "audio/aac" => "aac",
        "audio/mp4" => "m4a",
        "application/pdf" => "pdf",
        _ => "bin",
    };

    format!("{}.{}", default_filename, ext)
}

pub(super) fn build_sticker_preview_url(sticker_id: &str, sticker_resource_type: &str) -> String {
    if sticker_id.is_empty() {
        return String::new();
    }

    let asset = match sticker_resource_type {
        "ANIMATION" | "ANIMATION_SOUND" => "sticker_animation.png",
        "POPUP" | "POPUP_SOUND" => "sticker_popup.png",
        _ => "sticker.png",
    };

    format!(
        "https://stickershop.line-scdn.net/stickershop/v1/sticker/{}/android/{}",
        sticker_id, asset
    )
}

pub(super) async fn get_group_member_profile(
    client: &Client,
    access_token: &str,
    group_id: &str,
    user_id: &str,
) -> Result<MemberProfile, String> {
    let url = format!(
        "https://api.line.me/v2/bot/group/{}/member/{}",
        group_id, user_id
    );

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| format!("request error: {:?}", err))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(format!("LINE API returned {}: {}", status, body));
    }

    response
        .json::<MemberProfile>()
        .await
        .map_err(|err| format!("parse json error: {:?}", err))
}

pub(super) async fn get_room_member_profile(
    client: &Client,
    access_token: &str,
    room_id: &str,
    user_id: &str,
) -> Result<MemberProfile, String> {
    let url = format!(
        "https://api.line.me/v2/bot/room/{}/member/{}",
        room_id, user_id
    );

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| format!("request error: {:?}", err))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(format!("LINE API returned {}: {}", status, body));
    }

    response
        .json::<MemberProfile>()
        .await
        .map_err(|err| format!("parse json error: {:?}", err))
}

pub(super) async fn get_user_profile(
    client: &Client,
    access_token: &str,
    user_id: &str,
) -> Result<MemberProfile, String> {
    let url = format!("https://api.line.me/v2/bot/profile/{}", user_id);

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| format!("request error: {:?}", err))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(format!("LINE API returned {}: {}", status, body));
    }

    response
        .json::<MemberProfile>()
        .await
        .map_err(|err| format!("parse json error: {:?}", err))
}

#[derive(serde::Deserialize)]
struct GroupSummary {
    #[serde(default, rename = "groupName")]
    group_name: String,
}

#[derive(serde::Deserialize)]
struct RoomSummary {
    #[serde(default, rename = "roomName")]
    room_name: String,
}

pub(super) async fn get_group_name(
    client: &Client,
    access_token: &str,
    group_id: &str,
) -> Result<String, String> {
    let url = format!("https://api.line.me/v2/bot/group/{}/summary", group_id);

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| format!("request error: {:?}", err))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(format!("LINE API returned {}: {}", status, body));
    }

    response
        .json::<GroupSummary>()
        .await
        .map(|v| v.group_name)
        .map_err(|err| format!("parse json error: {:?}", err))
}

pub(super) async fn get_room_name(
    client: &Client,
    access_token: &str,
    room_id: &str,
) -> Result<String, String> {
    let url = format!("https://api.line.me/v2/bot/room/{}/summary", room_id);

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| format!("request error: {:?}", err))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(format!("LINE API returned {}: {}", status, body));
    }

    response
        .json::<RoomSummary>()
        .await
        .map(|v| v.room_name)
        .map_err(|err| format!("parse json error: {:?}", err))
}
