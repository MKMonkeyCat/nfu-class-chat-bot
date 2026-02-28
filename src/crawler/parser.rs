use crate::crawler::types::CrawlerBasePost;
use config::CrawlerEntryConfig;
use scraper::{ElementRef, Html, Selector};
use url::Url;

use super::types::CrawledPost;

pub(crate) fn parse_base_posts(
    entry: &CrawlerEntryConfig,
    body: &str,
) -> Result<Vec<CrawlerBasePost>, String> {
    let s = &entry.selectors;

    let item_sel =
        Selector::parse(&s.item_selector).map_err(|e| format!("invalid item_selector: {e}"))?;
    let id_sel =
        Selector::parse(&s.id_selector).map_err(|e| format!("invalid id_selector: {e}"))?;
    let title_sel =
        Selector::parse(&s.title_selector).map_err(|e| format!("invalid title_selector: {e}"))?;
    let link_sel =
        Selector::parse(&s.link_selector).map_err(|e| format!("invalid link_selector: {e}"))?;
    let time_sel =
        Selector::parse(&s.time_selector).map_err(|e| format!("invalid time_selector: {e}"))?;

    let category_sel = parse_opt_selector(s.category_selector.as_deref())?;
    let source_sel = parse_opt_selector(s.source_selector.as_deref())?;
    let tags_sel = parse_opt_selector(s.tags_selector.as_deref())?;
    let mut output = Vec::new();

    let document = Html::parse_document(body);
    for item in document.select(&item_sel) {
        let id = select_attr(&item, &id_sel, "href");
        let title = select_text(&item, &title_sel);
        let url = absolutize_url(&entry.url, &select_attr(&item, &link_sel, "href"));
        let time = select_text(&item, &time_sel);

        if id.is_empty() || title.is_empty() || url.is_empty() {
            continue;
        }

        let category = category_sel
            .as_ref()
            .map(|sel| select_text(&item, sel))
            .unwrap_or_default();

        let source_name = source_sel
            .as_ref()
            .map(|sel| select_text(&item, sel))
            .unwrap_or_else(|| entry.name.clone());

        let tags = tags_sel
            .as_ref()
            .map(|sel| {
                item.select(sel)
                    .map(|node| {
                        node.text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .filter(|tag| !tag.trim().is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        output.push(CrawlerBasePost {
            id,
            category,
            source_name,
            title,
            url,
            time,
            tags,
        });
    }

    Ok(output)
}

pub(crate) fn parse_full_post(
    entry: &CrawlerEntryConfig,
    body: &str,
) -> Result<CrawledPost, String> {
    let document = Html::parse_document(body);

    Ok(CrawledPost {
        id: String::new(),
        category: String::new(),
        source_name: entry.name.clone(),
        title: String::new(),
        url: String::new(),
        content: String::new(),
        time: String::new(),
        tags: Vec::new(),
    })
}

fn parse_opt_selector(sel_str: Option<&str>) -> Result<Option<Selector>, String> {
    match sel_str {
        Some(s) if !s.is_empty() => Selector::parse(s)
            .map(Some)
            .map_err(|e| format!("invalid selector '{s}': {e}")),
        _ => Ok(None),
    }
}

fn select_attr(item: &ElementRef<'_>, selector: &Selector, attr: &str) -> String {
    item.select(selector)
        .next()
        .and_then(|node| node.value().attr(attr))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn select_text(item: &ElementRef<'_>, selector: &Selector) -> String {
    item.select(selector)
        .next()
        .map(|node| {
            node.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn absolutize_url(base: &str, target: &str) -> String {
    if target.is_empty() {
        return String::new();
    }

    if let Ok(url) = Url::parse(target) {
        return url.to_string();
    }

    if let Ok(base_url) = Url::parse(base) {
        if let Ok(joined) = base_url.join(target) {
            return joined.to_string();
        }
    }

    target.to_string()
}
