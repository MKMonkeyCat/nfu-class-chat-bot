use crate::discord::{DiscordEmbedField, DiscordEmbedPayload};
use config::{CrawlerEntryConfig, NotificationConfig};
use serde::Serialize;
use utils::CompiledTemplate;

use super::types::{CrawledPost, LlmResult};

pub(crate) struct CrawlerDiscordMessage {
    pub(crate) webhook_username: Option<String>,
    pub(crate) content: Option<String>,
    pub(crate) embed: DiscordEmbedPayload,
}

#[derive(Serialize)]
struct CrawlerTemplateContext {
    entries_name: String,
    title: String,
    summary: String,
    analysis: String,
    analysis_cn: String,
    content: String,
    category: String,
    source_name: String,
    author_unit: String,
    url: String,
    time: String,
    tags: String,
    calendar: String,
    attachments: String,
}

pub(crate) fn build_discord_message(
    entry: &CrawlerEntryConfig,
    notification: &NotificationConfig,
    post: &CrawledPost,
    llm: &LlmResult,
) -> CrawlerDiscordMessage {
    let title = if llm.title.trim().is_empty() {
        post.title.clone()
    } else {
        llm.title.clone()
    };

    let summary = if llm.summary.trim().is_empty() {
        post.content.chars().take(120).collect::<String>()
    } else {
        llm.summary.clone()
    };

    let mut tags = post.tags.clone();
    for tag in &llm.tags {
        if !tag.trim().is_empty() && !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.clone());
        }
    }

    let tags_text = tags
        .iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| format!("#{tag}"))
        .collect::<Vec<_>>()
        .join(" ");

    let calendar_text = if llm.calendar.is_empty() {
        String::new()
    } else {
        llm.calendar
            .iter()
            .filter(|item| !item.date.trim().is_empty() || !item.event.trim().is_empty())
            .map(|item| format!("- {} {}", item.date.trim(), item.event.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let attachments_text = if post.attachments.is_empty() {
        String::new()
    } else {
        post.attachments.join("\n")
    };

    let context = CrawlerTemplateContext {
        entries_name: entry.name.clone(),
        title: title.clone(),
        summary: summary.clone(),
        analysis: llm.analysis.trim().to_string(),
        analysis_cn: llm.analysis_cn.trim().to_string(),
        content: post.content.clone(),
        category: post.category.clone(),
        source_name: post.source_name.clone(),
        author_unit: post.author_unit.clone(),
        url: post.url.clone(),
        time: post.time.clone(),
        tags: tags_text,
        calendar: calendar_text,
        attachments: attachments_text,
    };

    let embed_title = CompiledTemplate::compile(&notification.discord.embed_title).render(&context);

    let mut fields = notification
        .discord
        .fields
        .iter()
        .map(|field| DiscordEmbedField {
            name: CompiledTemplate::compile(&field.name).render(&context),
            value: CompiledTemplate::compile(&field.value).render(&context),
            inline: field.inline,
        })
        .filter(|field| !field.name.trim().is_empty() && !field.value.trim().is_empty())
        .collect::<Vec<_>>();

    if notification.discord.include_attachments && !post.attachments.is_empty() {
        fields.push(DiscordEmbedField {
            name: "附件".to_string(),
            value: post.attachments.join("\n"),
            inline: false,
        });
    }

    let webhook_username = if notification.discord.webhook_name_template.trim().is_empty() {
        None
    } else {
        Some(
            CompiledTemplate::compile(&notification.discord.webhook_name_template).render(&context),
        )
    };

    CrawlerDiscordMessage {
        webhook_username,
        content: None,
        embed: DiscordEmbedPayload {
            title: embed_title,
            description: summary,
            url: post.url.clone(),
            color: notification.discord.embed_color,
            fields,
            footer: if post.time.trim().is_empty() {
                String::new()
            } else {
                format!("時間：{}", post.time)
            },
        },
    }
}
