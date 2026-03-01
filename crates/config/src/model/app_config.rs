use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utils::DocReader;

/// 主應用程式設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct AppConfig {
    /// 角色權限設定
    #[serde(default)]
    pub roles: RoleConfig,

    /// 班級學生對應表 (key: 班級或學號, value: 學生名稱)
    #[serde(default)]
    pub class_students: HashMap<String, String>,

    /// 聊天整合設定（Line / Discord）
    #[serde(default)]
    pub link_chat: LinkChatConfig,

    /// 爬蟲系統設定
    #[serde(default)]
    pub crawler_system: CrawlerSystemConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut students = HashMap::new();
        students.insert("40000001".to_string(), "XXA".to_string());
        students.insert("40000002".to_string(), "XXB".to_string());
        students.insert("40000003".to_string(), "XXC".to_string());

        Self {
            roles: RoleConfig::default(),
            class_students: students,
            link_chat: LinkChatConfig::default(),
            crawler_system: CrawlerSystemConfig::default(),
        }
    }
}

/// 角色權限設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct RoleConfig {
    /// 管理員角色 ID 列表
    #[serde(default)]
    pub admin_roles: Vec<u64>,

    /// 本地角色 ID
    pub local_role: u64,

    /// 高級角色 ID
    pub senior_role: u64,

    /// 教師角色 ID
    pub teacher_role: u64,

    /// 訪客角色 ID
    pub guest_role: u64,

    /// 已驗證使用者角色 ID
    pub verified_role: u64,
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self {
            admin_roles: vec![1234567890123456789],
            local_role: 1234567890123456789,
            senior_role: 1234567890123456789,
            teacher_role: 1234567890123456789,
            guest_role: 1234567890123456789,
            verified_role: 1234567890123456789,
        }
    }
}

/// 聊天整合設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct LinkChatConfig {
    /// 是否啟用聊天整合
    #[serde(default)]
    pub enabled: bool,

    /// 綁定的伺服器地址 (IP:Port)
    #[serde(default = "default_bind")]
    pub bind: String,

    /// LINE channel secret
    #[serde(default)]
    pub line_channel_secret: String,

    /// LINE channel access token
    #[serde(default)]
    pub line_channel_access_token: String,

    /// 快取存活時間 (秒)
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,

    /// 群組對應表 (key: 群組 ID, value: 對應 Discord/Webhook 設定)
    #[serde(default)]
    pub group_mapping: HashMap<String, GroupRouteConfig>,
}

impl Default for LinkChatConfig {
    fn default() -> Self {
        let mut group_mapping = HashMap::new();
        group_mapping.insert(
            "*".to_string(),
            GroupRouteConfig {
                discord_channel_id: 1234567890123456789,
                discord_webhook_url: String::new(),
                discord_webhook_avatar_url: String::new(),
                message_template: "{name} ({group}): {message}".to_string(),
                webhook_avatar_url: String::new(),
                webhook_name_template: "{name} ({group})".to_string(),
                webhook_message_template: "{message}".to_string(),
            },
        );

        Self {
            enabled: true,
            bind: default_bind(),
            line_channel_secret: "demo_secret".to_string(),
            line_channel_access_token: "demo_access_token".to_string(),
            cache_ttl_seconds: default_cache_ttl(),
            group_mapping,
        }
    }
}

/// 群組路由設定，用於將訊息導向 Discord / Webhook
#[derive(Debug, Serialize, Deserialize, Clone, Default, DocReader)]
pub struct GroupRouteConfig {
    /// Discord 頻道 ID
    pub discord_channel_id: u64,

    /// Discord Webhook URL
    #[serde(default)]
    pub discord_webhook_url: String,

    /// Discord Webhook 頭像 URL
    #[serde(default)]
    pub discord_webhook_avatar_url: String,

    /// 訊息模板 (可選)
    #[serde(default)]
    pub message_template: String,

    /// Webhook 頭像 URL (可選)
    #[serde(default)]
    pub webhook_avatar_url: String,

    /// Webhook 名稱模板 (可選)
    #[serde(default)]
    pub webhook_name_template: String,

    /// Webhook 訊息模板 (可選)
    #[serde(default)]
    pub webhook_message_template: String,
}

/// 爬蟲系統設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct CrawlerSystemConfig {
    /// 爬蟲任務檔案路徑
    #[serde(default = "default_tasks_file")]
    pub tasks_file: String,
}

impl Default for CrawlerSystemConfig {
    fn default() -> Self {
        Self {
            tasks_file: default_tasks_file(),
        }
    }
}

/// 預設綁定地址
fn default_bind() -> String {
    "0.0.0.0:8080".to_string()
}

/// 預設快取 TTL (秒)
fn default_cache_ttl() -> u64 {
    300
}

/// 預設爬蟲任務檔案路徑
fn default_tasks_file() -> String {
    "config/crawlers.toml".to_string()
}
