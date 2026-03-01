use chrono::{DateTime, Utc};
use config::LinkChatConfig;
use serde::Serialize;
use utils::CompiledTemplate;

use super::types::DeliveryTarget;

#[derive(Serialize)]
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

    Some(CompiledTemplate::compile(&target.webhook_name_template).render(context))
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

    CompiledTemplate::compile(template).render(context)
}
