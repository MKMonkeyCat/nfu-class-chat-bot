mod delivery;
mod llm;
mod message;
mod parser;
mod schedule;
mod sim_hash;
mod types;
mod utils;

use crate::crawler::utils::crawler_prune_seen;
use chrono::{DateTime, Utc};
use config::{CrawlerConfig, CrawlerEntryConfig, CrawlerLlmConfig};
use entity::model::announcement;
use reqwest::Client;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
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

                match process_entry(&reqwest, &http, &db, entry, &cfg.llm).await {
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
        let message = build_discord_message(&post, &llm_result);
        if let Err(err) = send_to_discord(&reqwest, http, entry, &message).await {
            eprintln!("[crawler] {} failed to send message: {}", entry.name, err);
        } else {
            sent += 1;
        }
    }
    Ok(sent)
}
