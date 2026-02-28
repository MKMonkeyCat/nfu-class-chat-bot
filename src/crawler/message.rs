use super::types::{CrawledPost, LlmResult};

pub(crate) fn build_discord_message(post: &CrawledPost, llm: &LlmResult) -> String {
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

    let tag_line = if tags.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n{}",
            tags.iter()
                .map(|tag| format!("#{}", tag.trim()))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };

    let calendar_line = if llm.calendar.is_empty() {
        String::new()
    } else {
        let items = llm
            .calendar
            .iter()
            .filter(|item| !item.date.trim().is_empty() || !item.event.trim().is_empty())
            .map(|item| format!("- {} {}", item.date.trim(), item.event.trim()))
            .collect::<Vec<_>>();

        if items.is_empty() {
            String::new()
        } else {
            format!("\n\n**行事曆重點**\n{}", items.join("\n"))
        }
    };

    let analysis_line = if llm.analysis.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n**分析**\n{}", llm.analysis.trim())
    };

    format!(
        "**[{}] {}**\n{}\n{}\n{}{}{}",
        post.source_name,
        title,
        summary,
        if post.time.trim().is_empty() {
            "".to_string()
        } else {
            format!("時間：{}", post.time)
        },
        post.url,
        analysis_line,
        format!("{}{}", calendar_line, tag_line)
    )
}
