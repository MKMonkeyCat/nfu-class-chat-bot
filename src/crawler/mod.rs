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
use reqwest::Client;
use sea_orm::DatabaseConnection;
use serenity::all::Http;
use serenity::prelude::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use delivery::send_to_discord;
use llm::call_llm;
use message::build_discord_message;
use schedule::{FALLBACK_LOOP_SECONDS, SEEN_TTL_SECONDS, compute_next_run};
use types::LlmResult;

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

            if let Err(err) = crawler_prune_seen(&db, SEEN_TTL_SECONDS).await {
                eprintln!("[crawler] failed to prune seen keys: {}", err);
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
    for post in &base_posts {
        println!(
            "[crawler] {} found post: id='{}', title='{}'",
            entry.name, post.id, post.title
        );
    }

    // let mut posts = parse_posts(entry, &body)?;
    // if posts.is_empty() {
    //     return Ok(0);
    // }

    // posts.reverse();

    let mut sent = 0usize;
    // for post in posts {
    //     let llm_result = if llm_cfg.enabled {
    //         call_llm(reqwest, llm_cfg, &post).await.unwrap_or_default()
    //     } else {
    //         LlmResult {
    //             is_relevant: true,
    //             title: post.title.clone(),
    //             summary: post.content.chars().take(120).collect(),
    //             analysis: post.content.clone(),
    //             analysis_cn: post.content.clone(),
    //             calendar: Vec::new(),
    //             tags: Vec::new(),
    //         }
    //     };

    //     let content = build_discord_message(&post, &llm_result);
    //     send_to_discord(reqwest, http, entry, &content).await?;
    //     sent += 1;
    // }

    Ok(sent)
}
