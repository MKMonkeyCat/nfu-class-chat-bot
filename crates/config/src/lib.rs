mod build;
mod model;

pub use model::*;

use config::{Config, ConfigError, Environment, File};
use notify::{Event, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::{fs, time};
use toml::Value;

use crate::build::{generate_toml_with_comments, generate_toml_with_comments_from_value};

pub async fn load_all_configs() -> Result<(AppConfig, CrawlerConfig), ConfigError> {
    let config_dir = Path::new("config");

    if !config_dir.exists() {
        fs::create_dir_all(config_dir)
            .await
            .expect("Failed to create config directory");
    }

    ensure_config_file::<AppConfig>("config/app_config.toml").await?;
    ensure_config_file::<CrawlerConfig>("config/crawlers.toml").await?;

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

async fn ensure_config_file<T>(path: &str) -> Result<(), ConfigError>
where
    T: Default + Serialize + utils::DocReader,
{
    let p = Path::new(path);

    if !p.exists() {
        println!("Config file not found, creating default: {}", path);
        fs::write(p, generate_toml_with_comments::<T>())
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))?;
        return Ok(());
    }

    let existing_raw = fs::read_to_string(p)
        .await
        .map_err(|e| ConfigError::Foreign(Box::new(e)))?;

    let existing_value: Value =
        toml::from_str(&existing_raw).map_err(|e| ConfigError::Foreign(Box::new(e)))?;
    let mut merged_value =
        Value::try_from(&T::default()).map_err(|e| ConfigError::Foreign(Box::new(e)))?;

    merge_toml_values(&mut merged_value, &existing_value);

    let merged_content = generate_toml_with_comments_from_value::<T>(&merged_value);
    if existing_raw.trim() != merged_content.trim() {
        fs::write(p, merged_content)
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))?;
        println!("Config file auto-updated with new defaults: {}", path);
    }

    Ok(())
}

fn merge_toml_values(defaults: &mut Value, existing: &Value) {
    match (defaults, existing) {
        (Value::Table(default_table), Value::Table(existing_table)) => {
            for (key, existing_value) in existing_table {
                if let Some(default_value) = default_table.get_mut(key) {
                    merge_toml_values(default_value, existing_value);
                } else {
                    default_table.insert(key.clone(), existing_value.clone());
                }
            }
        }
        (Value::Array(default_arr), Value::Array(existing_arr)) => {
            let is_table_array = |arr: &[Value]| arr.iter().all(Value::is_table);

            if is_table_array(default_arr) && is_table_array(existing_arr) {
                let template = default_arr.first().cloned();
                let mut merged_items = Vec::with_capacity(existing_arr.len());

                for existing_item in existing_arr {
                    let mut merged_item = template.clone().unwrap_or_else(|| existing_item.clone());
                    merge_toml_values(&mut merged_item, existing_item);
                    merged_items.push(merged_item);
                }

                *default_arr = merged_items;
            } else {
                *default_arr = existing_arr.clone();
            }
        }
        (default_value, existing_value) => {
            *default_value = existing_value.clone();
        }
    }
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
