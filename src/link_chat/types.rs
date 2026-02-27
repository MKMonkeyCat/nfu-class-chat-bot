use crate::app_config::AppConfig;
use reqwest::Client;
use serde::Deserialize;
use serenity::all::Http;
use serenity::prelude::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock as TokioRwLock;

#[derive(Clone)]
pub(super) struct CacheEntry<T> {
    pub(super) value: T,
    pub(super) expires_at: Instant,
}

#[derive(Clone)]
pub(super) struct LinkChatState {
    pub(super) config: Arc<RwLock<AppConfig>>,
    pub(super) http: Arc<Http>,
    pub(super) reqwest: Client,
    pub(super) profile_cache: Arc<TokioRwLock<HashMap<String, CacheEntry<MemberProfile>>>>,
    pub(super) group_name_cache: Arc<TokioRwLock<HashMap<String, CacheEntry<String>>>>,
    pub(super) room_name_cache: Arc<TokioRwLock<HashMap<String, CacheEntry<String>>>>,
}

impl LinkChatState {
    pub(super) fn with_defaults(config: Arc<RwLock<AppConfig>>, http: Arc<Http>) -> Self {
        Self {
            config,
            http,
            reqwest: Client::new(),
            profile_cache: Arc::new(TokioRwLock::new(HashMap::new())),
            group_name_cache: Arc::new(TokioRwLock::new(HashMap::new())),
            room_name_cache: Arc::new(TokioRwLock::new(HashMap::new())),
        }
    }

    pub(super) async fn get_cached_profile(&self, key: &str) -> Option<MemberProfile> {
        let cache = self.profile_cache.read().await;
        cache.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub(super) async fn set_cached_profile(&self, key: String, value: MemberProfile) {
        let ttl = self.cache_ttl().await;
        let mut cache = self.profile_cache.write().await;
        cache.insert(
            key,
            CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    pub(super) async fn get_cached_group_name(&self, key: &str) -> Option<String> {
        let cache = self.group_name_cache.read().await;
        cache.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub(super) async fn set_cached_group_name(&self, key: String, value: String) {
        let ttl = self.cache_ttl().await;
        let mut cache = self.group_name_cache.write().await;
        cache.insert(
            key,
            CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    pub(super) async fn get_cached_room_name(&self, key: &str) -> Option<String> {
        let cache = self.room_name_cache.read().await;
        cache.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub(super) async fn set_cached_room_name(&self, key: String, value: String) {
        let ttl = self.cache_ttl().await;
        let mut cache = self.room_name_cache.write().await;
        cache.insert(
            key,
            CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    async fn cache_ttl(&self) -> Duration {
        let cfg = self.config.read().await;
        Duration::from_secs(cfg.link_chat.cache_ttl_seconds.max(1))
    }
}

#[derive(Clone)]
pub(super) struct DeliveryTarget {
    pub(super) discord_channel_id: u64,
    pub(super) discord_webhook_url: String,
    pub(super) discord_webhook_avatar_url: String,
    pub(super) message_template: String,
    pub(super) webhook_name_template: String,
    pub(super) webhook_message_template: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct LineWebhookPayload {
    #[serde(default)]
    pub(super) events: Vec<LineEvent>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LineEvent {
    #[serde(rename = "type")]
    pub(super) event_type: String,
    #[serde(default)]
    pub(super) timestamp: i64,
    pub(super) source: LineSource,
    #[serde(default)]
    pub(super) message: Option<LineMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(super) enum LineSource {
    User {
        #[serde(rename = "userId")]
        user_id: String,
    },
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
        #[serde(default, rename = "userId")]
        user_id: String,
    },
    Room {
        #[serde(rename = "roomId")]
        room_id: String,
        #[serde(default, rename = "userId")]
        user_id: String,
    },
}

impl LineSource {
    pub(super) fn source_id(&self) -> &str {
        match self {
            LineSource::User { user_id } => user_id,
            LineSource::Group { group_id, .. } => group_id,
            LineSource::Room { room_id, .. } => room_id,
        }
    }

    pub(super) fn sender_user_id(&self) -> Option<&str> {
        match self {
            LineSource::User { user_id } => Some(user_id),
            LineSource::Group { user_id, .. } => {
                if user_id.is_empty() {
                    None
                } else {
                    Some(user_id)
                }
            }
            LineSource::Room { user_id, .. } => {
                if user_id.is_empty() {
                    None
                } else {
                    Some(user_id)
                }
            }
        }
    }

    pub(super) fn group_id(&self) -> Option<&str> {
        match self {
            LineSource::Group { group_id, .. } => Some(group_id),
            _ => None,
        }
    }

    pub(super) fn room_id(&self) -> Option<&str> {
        match self {
            LineSource::Room { room_id, .. } => Some(room_id),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(super) enum LineMessage {
    Text {
        #[serde(default)]
        text: String,
    },
    Image {
        #[serde(default, rename = "id")]
        message_id: String,
    },
    Video {
        #[serde(default, rename = "id")]
        message_id: String,
    },
    Audio {
        #[serde(default, rename = "id")]
        message_id: String,
    },
    File {
        #[serde(default, rename = "id")]
        message_id: String,
        #[serde(default, rename = "fileName")]
        file_name: String,
        // #[serde(default, rename = "fileSize")]
        // file_size: u64,
    },
    Location {
        #[serde(default)]
        title: String,
        #[serde(default)]
        address: String,
        #[serde(default)]
        latitude: f64,
        #[serde(default)]
        longitude: f64,
    },
    Sticker {
        #[serde(default, rename = "stickerId")]
        sticker_id: String,
        #[serde(default, rename = "stickerResourceType")]
        sticker_resource_type: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct MemberProfile {
    #[serde(default, rename = "displayName")]
    pub(super) display_name: String,
    #[serde(default, rename = "pictureUrl")]
    pub(super) picture_url: String,
}
