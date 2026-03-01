use crate::crawler::types::CrawlerBasePost;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use config::CrawlerEntryConfig;
use scraper::{ElementRef, Html, Selector};
use url::Url;
use utils::app_timezone;

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
        let time = select_text(&item, &time_sel).replace("寫於", "");

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
            category,
            source_name,
            title,
            url,
            time: parse_time_string(&time).unwrap_or_else(|_| app_timezone().now_utc()),
            tags,
        });
    }

    Ok(output)
}

pub(crate) fn parse_full_post(
    entry: &CrawlerEntryConfig,
    base_post: &CrawlerBasePost,
    body: &str,
) -> Result<CrawledPost, String> {
    let document = Html::parse_document(body);

    let source_name = if base_post.source_name.trim().is_empty() {
        entry.name.clone()
    } else {
        base_post.source_name.clone()
    };

    let sub_selectors = match &entry.sub_selectors {
        Some(s) => s,
        None => {
            return Ok(CrawledPost {
                category: base_post.category.clone(),
                source_name: source_name.clone(),
                title: base_post.title.clone(),
                url: base_post.url.clone(),
                content: String::new(),
                author_unit: String::new(),
                attachments: Vec::new(),
                time: base_post.time,
                tags: base_post.tags.clone(),
            });
        }
    };

    let author_sel = Selector::parse(&sub_selectors.author_unit)
        .map_err(|e| format!("invalid author_unit selector: {e}"))?;
    let content_sel = Selector::parse(&sub_selectors.full_content)
        .map_err(|e| format!("invalid full_content selector: {e}"))?;
    let attachments_sel = Selector::parse(&sub_selectors.attachments)
        .map_err(|e| format!("invalid attachments selector: {e}"))?;

    let content = select_text(&document.root_element(), &content_sel);
    let author_unit = select_text(&document.root_element(), &author_sel);
    let attachments = document
        .select(&attachments_sel)
        .filter_map(|node| node.value().attr("href"))
        .map(|href| absolutize_url(&base_post.url, href))
        .filter(|url| !url.is_empty())
        .collect::<Vec<_>>();

    Ok(CrawledPost {
        category: base_post.category.clone(),
        source_name: source_name.clone(),
        title: base_post.title.clone(),
        url: base_post.url.clone(),
        content,
        author_unit,
        attachments,
        time: base_post.time,
        tags: base_post.tags.clone(),
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

fn parse_time_string(time_str: &str) -> Result<DateTime<Utc>, String> {
    let tz = app_timezone();
    let mut s = time_str.trim().to_string();

    if let Some(rest) = s.strip_prefix("寫於") {
        s = rest.trim().to_string();
    }

    for week in &[
        "週一,",
        "週二,",
        "週三,",
        "週四,",
        "週五,",
        "週六,",
        "週日,",
        "星期一,",
        "星期二,",
        "星期三,",
        "星期四,",
        "星期五,",
        "星期六,",
        "星期日,",
    ] {
        s = s.replace(week, "");
    }

    for (cn, num) in &[
        ("十二月", "12"),
        ("十一月", "11"),
        ("十月", "10"),
        ("九月", "09"),
        ("八月", "08"),
        ("七月", "07"),
        ("六月", "06"),
        ("五月", "05"),
        ("四月", "04"),
        ("三月", "03"),
        ("二月", "02"),
        ("一月", "01"),
    ] {
        s = s.replace(cn, num);
    }

    s = s.replace("　", " ");
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");

    const FORMATS: &[&str] = &[
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%m %d, %Y",
        "%m %e, %Y",
        "%m %d %Y",
        "%m %e %Y",
        "%b %d, %Y",
        "%b %e, %Y",
        "%B %d, %Y",
        "%B %e, %Y",
        "%b %d %Y",
        "%b %e %Y",
        "%B %d %Y",
        "%B %e %Y",
        "%d %m %Y %H:%M",
        "%d %m %Y",
    ];

    for &fmt in FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(&s, fmt) {
            if let Some(utc) = tz.naive_local_to_utc(dt) {
                return Ok(utc);
            }
        }
        if let Ok(d) = NaiveDate::parse_from_str(&s, fmt) {
            if let Some(dt) = d.and_hms_opt(0, 0, 0)
                && let Some(utc) = tz.naive_local_to_utc(dt)
            {
                return Ok(utc);
            }
        }
    }

    eprintln!("Warning: unable to parse '{}'", time_str);
    Err(time_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_string() {
        let cases = vec![
            ("寫於2023-03-05 14:30:00", "2023-03-05T06:30:00+00:00"),
            ("2023/03/05 14:30", "2023-03-05T06:30:00+00:00"),
            ("2023-03-05", "2023-03-04T16:00:00+00:00"),
            ("三月 5, 2023", "2023-03-04T16:00:00+00:00"),
            ("March 5, 2023", "2023-03-04T16:00:00+00:00"),
            ("5 3 2023 14:30", "2023-03-05T06:30:00+00:00"),
            (" 週四, 28 十一月 2024 13:53", "2024-11-28T05:53:00+00:00"),
        ];

        for (input, expected) in cases {
            let parsed = parse_time_string(input).expect("should parse");
            assert_eq!(parsed.to_rfc3339(), expected);
        }
    }
}
