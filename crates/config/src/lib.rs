mod build;
mod model;

pub use model::*;

use config::{Config, ConfigError, Environment, File};
use notify::{Event, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::{fs, time};

use crate::build::generate_toml_with_comments;

pub async fn load_all_configs() -> Result<(AppConfig, CrawlerConfig), ConfigError> {
    let config_dir = Path::new("config");

    if !config_dir.exists() {
        fs::create_dir_all(config_dir)
            .await
            .expect("Failed to create config directory");
    }

    let configs = [
        (
            "config/app_config.toml",
            generate_toml_with_comments::<AppConfig>(),
        ),
        (
            "config/crawlers.toml",
            generate_toml_with_comments::<CrawlerConfig>(),
        ),
    ];

    for (path, content) in configs {
        let p = Path::new(path);
        if !p.exists() {
            println!("Config file not found, creating default: {}", path);
            fs::write(p, content)
                .await
                .map_err(|e| ConfigError::Foreign(Box::new(e)))?;
        }
    }

    let app_cfg: AppConfig = Config::builder()
        .add_source(File::with_name("config/app_config"))
        .add_source(Environment::with_prefix("APP").separator("__"))
        .build()?
        .try_deserialize()?;

    let crawler_cfg: CrawlerConfig = Config::builder()
        .add_source(File::with_name("config/crawlers"))
        .add_source(Environment::with_prefix("CRAWLERS").separator("__"))
        .build()?
        .try_deserialize()?;

    Ok((app_cfg, crawler_cfg))
}

pub fn spawn_config_hot_reload(
    app_cfg: Arc<RwLock<AppConfig>>,
    crawler_cfg: Arc<RwLock<CrawlerConfig>>,
) {
    tokio::spawn(async move {
        let (tx, mut rx) = mpsc::channel(1);

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    let _ = tx.try_send(());
                }
            }
        })
        .expect("Failed to create watcher");

        let config_dir = Path::new("config");
        if let Err(e) = watcher.watch(config_dir, RecursiveMode::NonRecursive) {
            eprintln!("Watcher error: {}", e);
        }

        while let Some(_) = rx.recv().await {
            time::sleep(Duration::from_millis(500)).await;
            while let Ok(_) = rx.try_recv() {}

            match load_all_configs().await {
                Ok((new_app, new_crawler)) => {
                    let mut app_lock = app_cfg.write().await;
                    let mut crawler_lock = crawler_cfg.write().await;
                    *app_lock = new_app;
                    *crawler_lock = new_crawler;
                    println!("Config & Crawlers reloaded successfully.");
                }
                Err(e) => eprintln!("Reload failed: {}", e),
            }
        }
    });
}
