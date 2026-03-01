mod delivery;
mod llm;
mod message;
mod parser;
mod schedule;
mod sim_hash;
mod types;
mod utils;

use crate::crawler::sim_hash::FingerprintEngine;
use crate::crawler::utils::crawler_prune_seen;
use chrono::{DateTime, Utc};
use config::{CrawlerConfig, CrawlerEntryConfig, CrawlerLlmConfig, NotificationConfig};
use entity::model::announcement;
use reqwest::Client;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use serenity::all::Http;
use serenity::prelude::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use delivery::send_to_discord;
use llm::call_llm;
use message::build_discord_message;
use schedule::{FALLBACK_LOOP_SECONDS, SEEN_TTL_SECONDS, compute_next_run};

pub fn spawn_crawler(config: Arc<RwLock<CrawlerConfig>>, http: Arc<Http>, db: DatabaseConnection) {
    tokio::spawn(async move {
        let reqwest = Client::new();
        let mut next_run: HashMap<String, DateTime<Utc>> = HashMap::new();

        loop {
            let cfg = {
                let guard = config.read().await;
                guard.clone()
            };

            if !cfg.enabled || cfg.entries.is_empty() {
                tokio::time::sleep(Duration::from_secs(FALLBACK_LOOP_SECONDS)).await;
                continue;
            }

            let now = Utc::now();
            for entry in &cfg.entries {
                let key = format!("{}::{}", entry.name, entry.url);
                let due = next_run.get(&key).cloned().unwrap_or(now);
                if now < due {
                    continue;
                }

                if let Err(err) = crawler_prune_seen(&db, SEEN_TTL_SECONDS).await {
                    eprintln!("[crawler] failed to prune seen keys: {}", err);
                }

                match process_entry(&reqwest, &http, &db, entry, &cfg.llm, &cfg.notifications).await
                {
                    Ok(sent) if sent > 0 => {
                        println!("[crawler] {} pushed {} post(s)", entry.name, sent);
                    }
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("[crawler] {} failed: {}", entry.name, err);
                    }
                }

                next_run.insert(key, compute_next_run(&entry.cron, now));
            }

            tokio::time::sleep(Duration::from_secs(FALLBACK_LOOP_SECONDS)).await;
        }
    });
}

async fn process_entry(
    reqwest: &Client,
    http: &Arc<Http>,
    db: &DatabaseConnection,
    entry: &CrawlerEntryConfig,
    llm_cfg: &CrawlerLlmConfig,
    notifications: &HashMap<String, NotificationConfig>,
) -> Result<usize, String> {
    let timeout = Duration::from_millis(entry.timeout_ms.max(1000));
    let response = reqwest
        .get(&entry.url)
        .header("User-Agent", entry.config.user_agent.as_str())
        .timeout(timeout)
        .send()
        .await
        .map_err(|err| format!("fetch failed: {err}"))?;

    let body = response
        .text()
        .await
        .map_err(|err| format!("read body failed: {err}"))?;

    let base_posts = parser::parse_base_posts(entry, &body)?;
    let mut base_posts = base_posts
        .into_iter()
        .filter(|post| !post.url.is_empty())
        .collect::<Vec<_>>();

    let base_posts_urls = base_posts
        .iter()
        .map(|post| post.url.clone())
        .collect::<Vec<_>>();

    let existing_announcements: Vec<announcement::Model> = announcement::Entity::find()
        .filter(announcement::Column::Url.is_in(base_posts_urls))
        .all(db)
        .await
        .map_err(|err| format!("db query failed: {err}"))?;

    let existing_urls = existing_announcements
        .into_iter()
        .map(|a| a.url)
        .collect::<HashSet<_>>();

    base_posts.retain(|post| !existing_urls.contains(&post.url));

    let engine = FingerprintEngine::new();
    let mut sent = 0usize;
    for post in &base_posts {
        println!("[crawler] {} found new post: '{:?}'", entry.name, post);

        let timeout = Duration::from_millis(entry.timeout_ms.max(1000));
        let response = reqwest
            .get(&post.url)
            .header("User-Agent", entry.config.user_agent.as_str())
            .timeout(timeout)
            .send()
            .await
            .map_err(|err| format!("fetch failed: {err}"))?;

        let body = response
            .text()
            .await
            .map_err(|err| format!("read body failed: {err}"))?;

        let post = match parser::parse_full_post(entry, post, &body) {
            Ok(c) => c,
            Err(err) => {
                eprintln!(
                    "[crawler] {} failed to fetch full content for post '{}': {}",
                    entry.name, post.title, err
                );
                continue;
            }
        };

        println!(
            "[crawler] {} fetched full content for post '{:?}'",
            entry.name, post
        );

        let llm_result = call_llm(&reqwest, llm_cfg, &post).await?;
        println!("[LLM] {} result: {:?}", entry.name, llm_result);

        let fp = engine.generate(&llm_result.analysis_cn);
        let chunks = FingerprintEngine::split_fingerprint(fp);
        let threshold = 16;

        let candidates = announcement::Entity::find()
            .filter(
                Condition::any()
                    .add(announcement::Column::Chunk0.eq(chunks[0]))
                    .add(announcement::Column::Chunk1.eq(chunks[1]))
                    .add(announcement::Column::Chunk2.eq(chunks[2]))
                    .add(announcement::Column::Chunk3.eq(chunks[3])),
            )
            .all(db)
            .await
            .map_err(|err| format!("db query failed: {err}"))?;

        let mut is_similar = false;
        for candidate in &candidates {
            let cand_fp = candidate.simhash.parse::<u64>().unwrap_or(0);
            if cand_fp == 0 {
                continue;
            }

            let dist = FingerprintEngine::hamming_distance(fp, cand_fp);
            if dist <= threshold {
                is_similar = true;
                break;
            }
        }

        let new_announcement = announcement::ActiveModel {
            id: Default::default(), // auto-increment
            category: Set(post.category.clone()),
            source_name: Set(post.source_name.clone()),
            title: Set(post.title.clone()),
            url: Set(post.url.clone()),
            content: Set(post.content.clone()),
            time: Set(post.time.clone()),
            tags: Set(announcement::TagList(post.tags.clone())),

            implementation_at: Set(Utc::now()),
            created_at: Set(Utc::now()),

            simhash: Set(fp.to_string()),
            chunk0: Set(chunks[0]),
            chunk1: Set(chunks[1]),
            chunk2: Set(chunks[2]),
            chunk3: Set(chunks[3]),
        };

        if let Err(err) = announcement::Entity::insert(new_announcement)
            .exec(db)
            .await
        {
            eprintln!(
                "[crawler] {} failed to save announcement: {}",
                entry.name, err
            );
        }

        if !is_similar {
            let targets = selected_notifications(entry, notifications);
            if targets.is_empty() {
                println!(
                    "[crawler] {} skip notification for '{}' (target not matched)",
                    entry.name, post.title
                );
                continue;
            }

            let mut delivered = false;
            for notification in targets {
                let message = build_discord_message(entry, notification, &post, &llm_result);
                if let Err(err) = send_to_discord(reqwest, http, notification, &message).await {
                    eprintln!(
                        "[crawler] {} failed to send Discord message: {}",
                        entry.name, err
                    );
                } else {
                    delivered = true;
                }
            }

            if delivered {
                sent += 1;
            }
        } else {
            println!(
                "[crawler] {} found similar post for '{}', skipping Discord message",
                entry.name, post.title
            );
        }
    }
    Ok(sent)
}

fn selected_notifications<'a>(
    entry: &CrawlerEntryConfig,
    notifications: &'a HashMap<String, NotificationConfig>,
) -> Vec<&'a NotificationConfig> {
    if entry.notify_targets.is_empty() {
        return notifications.values().filter(|n| n.enabled).collect();
    }

    entry
        .notify_targets
        .iter()
        .filter_map(|target_id| notifications.get(target_id))
        .filter(|notification| notification.enabled)
        .collect()
}
