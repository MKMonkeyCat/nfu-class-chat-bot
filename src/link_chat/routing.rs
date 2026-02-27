use crate::app_config::LinkChatConfig;
use chrono::{DateTime, Utc};

use super::types::DeliveryTarget;

pub(super) struct TemplateContext<'a> {
    pub(super) name: &'a str,
    pub(super) avatar: &'a str,
    pub(super) user_id: &'a str,
    pub(super) group_name: &'a str,
    pub(super) group_id: &'a str,
    pub(super) message: &'a str,
    pub(super) timestamp: DateTime<Utc>,
}

pub(super) fn resolve_target(
    link_chat: &LinkChatConfig,
    source_group_id: &str,
) -> Option<DeliveryTarget> {
    let mapped = if source_group_id.is_empty() {
        None
    } else {
        link_chat
            .group_mapping
            .get(source_group_id)
            .or_else(|| link_chat.group_mapping.get("*"))
    }?;

    let webhook_url = mapped.discord_webhook_url.trim().to_string();
    let channel_id = mapped.discord_channel_id;
    if webhook_url.is_empty() && channel_id == 0 {
        return None;
    }

    Some(DeliveryTarget {
        discord_channel_id: channel_id,
        discord_webhook_url: webhook_url,
        discord_webhook_avatar_url: mapped.discord_webhook_avatar_url.trim().to_string(),
        message_template: mapped.message_template.clone(),
        webhook_name_template: mapped.webhook_name_template.clone(),
        webhook_message_template: mapped.webhook_message_template.clone(),
    })
}

pub(super) fn render_webhook_name(
    target: &DeliveryTarget,
    context: &TemplateContext,
) -> Option<String> {
    if target.discord_webhook_url.is_empty() {
        return None;
    }

    if target.webhook_name_template.trim().is_empty() {
        return Some(context.name.to_string());
    }

    Some(apply_template(&target.webhook_name_template, context))
}

pub(super) fn render_message_text(target: &DeliveryTarget, context: &TemplateContext) -> String {
    let template = if !target.discord_webhook_url.is_empty()
        && !target.webhook_message_template.trim().is_empty()
    {
        &target.webhook_message_template
    } else if !target.message_template.trim().is_empty() {
        &target.message_template
    } else {
        "{name} ({group}): {message}"
    };

    apply_template(template, context)
}

fn apply_template(template: &str, context: &TemplateContext) -> String {
    let mut result = String::with_capacity(template.len() + 32);
    let chars: Vec<char> = template.chars().collect();
    let mut cursor = 0;

    while cursor < chars.len() {
        if chars[cursor] != '{' {
            result.push(chars[cursor]);
            cursor += 1;
            continue;
        }

        let mut end = cursor + 1;
        while end < chars.len() && chars[end] != '}' {
            end += 1;
        }

        if end >= chars.len() {
            result.push(chars[cursor]);
            cursor += 1;
            continue;
        }

        let token: String = chars[cursor + 1..end].iter().collect();
        let rendered = render_token(&token, context).unwrap_or_else(|| format!("{{{}}}", token));
        result.push_str(&rendered);
        cursor = end + 1;
    }

    result
}

fn render_token(token: &str, context: &TemplateContext) -> Option<String> {
    let (key, fmt) = match token.split_once(':') {
        Some((key, fmt)) => (key.trim(), Some(fmt.trim())),
        None => (token.trim(), None),
    };

    let base = match key {
        "name" => context.name.to_string(),
        "avatar" => context.avatar.to_string(),
        "user_id" => context.user_id.to_string(),
        "group" => context.group_name.to_string(),
        "group_id" => context.group_id.to_string(),
        "message" => context.message.to_string(),
        "timestamp" => {
            if let Some(f) = fmt {
                if f.starts_with('%') {
                    return Some(context.timestamp.format(f).to_string());
                }
            }
            context.timestamp.to_rfc3339()
        }
        _ => return None,
    };

    Some(apply_string_format(base, fmt))
}

fn apply_string_format(mut value: String, fmt: Option<&str>) -> String {
    let Some(mut fmt) = fmt else {
        return value;
    };

    if fmt.is_empty() || fmt.starts_with('%') {
        return value;
    }

    let mut align = '<';
    if let Some(first) = fmt.chars().next() {
        if first == '<' || first == '>' || first == '^' {
            align = first;
            fmt = &fmt[first.len_utf8()..];
        }
    }

    let mut width_str = String::new();
    let mut precision_str = String::new();
    let mut seen_dot = false;

    for ch in fmt.chars() {
        if ch == '.' {
            seen_dot = true;
            continue;
        }
        if !ch.is_ascii_digit() {
            continue;
        }
        if seen_dot {
            precision_str.push(ch);
        } else {
            width_str.push(ch);
        }
    }

    if let Ok(precision) = precision_str.parse::<usize>() {
        value = value.chars().take(precision).collect();
    }

    let width = width_str.parse::<usize>().unwrap_or(0);
    let current = value.chars().count();
    if width <= current {
        return value;
    }

    let pad = width - current;
    match align {
        '>' => format!("{}{}", " ".repeat(pad), value),
        '^' => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{}{}", " ".repeat(left), value, " ".repeat(right))
        }
        _ => format!("{}{}", value, " ".repeat(pad)),
    }
}
