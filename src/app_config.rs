use config::{Config, ConfigError, File};
use notify::{Event, RecursiveMode, Watcher};
use serde::Deserialize;
use serenity::prelude::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub roles: RoleConfig,

    #[serde(default)]
    pub class_students: HashMap<String, String>,

    #[serde(default)]
    pub link_chat: LinkChatConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoleConfig {
    #[serde(default)]
    pub admin_roles: Vec<u64>,

    pub local_role: u64,
    pub senior_role: u64,
    pub teacher_role: u64,
    pub guest_role: u64,
    pub verified_role: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LinkChatConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub bind: String,

    #[serde(default)]
    pub line_channel_secret: String,

    #[serde(default)]
    pub line_channel_access_token: String,

    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,

    #[serde(default)]
    pub group_mapping: HashMap<String, GroupRouteConfig>,
}

impl Default for LinkChatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: "0.0.0.0:8080".to_string(),
            line_channel_secret: String::new(),
            line_channel_access_token: String::new(),
            cache_ttl_seconds: default_cache_ttl_seconds(),
            group_mapping: HashMap::new(),
        }
    }
}

fn default_cache_ttl_seconds() -> u64 {
    300
}

#[derive(Debug, Deserialize, Clone)]
pub struct GroupRouteConfig {
    #[serde(default, alias = "dc_channel_id")]
    pub discord_channel_id: u64,

    #[serde(default, alias = "webhook_url")]
    pub discord_webhook_url: String,

    #[serde(default, alias = "webhook_avatar_url")]
    pub discord_webhook_avatar_url: String,

    #[serde(default, alias = "message_template")]
    pub message_template: String,

    #[serde(default)]
    pub webhook_name_template: String,

    #[serde(default, alias = "webhook_message_template")]
    pub webhook_message_template: String,
}

impl Default for GroupRouteConfig {
    fn default() -> Self {
        Self {
            discord_channel_id: 0,
            discord_webhook_url: String::new(),
            discord_webhook_avatar_url: String::new(),
            message_template: "{name} ({group}): {message}".to_string(),
            webhook_name_template: "{name} ({group})".to_string(),
            webhook_message_template: "{message}".to_string(),
        }
    }
}

pub fn load_app_config() -> Result<AppConfig, ConfigError> {
    let settings = Config::builder()
        .add_source(File::with_name("config"))
        .build()?;
    settings.try_deserialize()
}

pub fn spawn_config_hot_reload(config: Arc<RwLock<AppConfig>>) {
    tokio::spawn(async move {
        let (tx, mut rx) = mpsc::channel(1);
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    let _ = tx.try_send(());
                }
            }
        })
        .expect("create file watcher failed");

        watcher
            .watch(Path::new("config.toml"), RecursiveMode::NonRecursive)
            .expect("listening config.toml failed");

        while let Some(_) = rx.recv().await {
            time::sleep(Duration::from_millis(100)).await;

            while let Ok(_) = rx.try_recv() {}
            if let Ok(new_cfg) = load_app_config() {
                let mut write_guard = config.write().await;
                *write_guard = new_cfg;
                println!("Updated config: config.toml");
            }
        }
    });
}
