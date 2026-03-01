use serde::{Deserialize, Serialize};
use utils::DocReader;

/// 主爬蟲設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct CrawlerConfig {
    /// 是否啟用爬蟲
    #[serde(default)]
    pub enabled: bool,

    /// 全域的 User-Agent 字串，若 entry 未設定則使用此值
    pub global_user_agent: String,

    /// LLM（大型語言模型）相關設定，用於內容分析或摘要
    pub llm: CrawlerLlmConfig,

    /// 各個爬蟲入口的設定
    pub entries: Vec<CrawlerEntryConfig>,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            global_user_agent: "NFU-Bot/1.0 (Github:MKMonkeyCat/nfu-class-chat-bot)".to_string(),
            llm: CrawlerLlmConfig::default(),
            entries: vec![CrawlerEntryConfig::default()],
        }
    }
}

/// 單一爬蟲入口設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct CrawlerEntryConfig {
    /// 該爬蟲入口名稱
    pub name: String,

    /// 該爬蟲入口是否啟用
    pub enabled: bool,

    /// 爬蟲入口 URL
    pub url: String,

    /// 爬蟲排程 Cron 表達式
    pub cron: String,

    /// 每次爬蟲執行的超時時間（毫秒）
    pub timeout_ms: u64,

    /// 入口的執行細節設定
    pub config: EntryRunConfig,

    /// 項目選取器設定，用於抓取列表頁資料
    pub selectors: SelectionConfig,

    /// 子內容選取器設定，用於抓取詳細頁資料
    pub sub_selectors: Option<SubSelectionConfig>,

    /// 通知設定，用於爬蟲完成後推播通知
    pub notifications: NotificationConfig,
}

impl Default for CrawlerEntryConfig {
    fn default() -> Self {
        Self {
            name: "虎科大-主頁".to_string(),
            enabled: true,
            url: "https://www.nfu.edu.tw/".to_string(),
            cron: "0 0 * * * *".to_string(), // every hour
            timeout_ms: 20_000,              // 20s
            config: EntryRunConfig::default(),
            selectors: SelectionConfig::default(),
            sub_selectors: Some(SubSelectionConfig::default()),
            notifications: NotificationConfig::default(),
        }
    }
}

/// 入口執行的設定細節
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct EntryRunConfig {
    /// 爬蟲深度（抓取頁面層數）
    pub depth: u32,

    /// 是否追蹤鏈結繼續抓取
    pub go_lnk: bool,

    /// 每次執行的最大抓取項目數
    pub max_items_per_run: u32,

    /// 是否啟用增量更新模式（只抓取新資料）
    pub incremental_update: bool,

    /// 該入口使用的 User-Agent
    pub user_agent: String,
}

impl Default for EntryRunConfig {
    fn default() -> Self {
        Self {
            depth: 2,
            go_lnk: true,
            max_items_per_run: 50,
            incremental_update: true,
            user_agent: "NFU-Bot/1.0 (Github:MKMonkeyCat/nfu-class-chat-bot)".to_string(),
        }
    }
}

/// 項目選取器設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct SelectionConfig {
    /// 項目列表選取器（CSS selector）
    pub item_selector: String,

    /// 項目唯一 ID 選取器
    pub id_selector: String,

    /// 類別選取器（可選）
    pub category_selector: Option<String>,

    /// 來源選取器（可選）
    pub source_selector: Option<String>,

    /// 標題選取器
    pub title_selector: String,

    /// 連結選取器
    pub link_selector: String,

    /// 發布時間選取器
    pub time_selector: String,

    /// 標籤選取器（可選）
    pub tags_selector: Option<String>,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            item_selector: ".w-annc".to_string(),
            id_selector: "a".to_string(),
            category_selector: None,
            source_selector: None,
            title_selector: ".w-annc__title".to_string(),
            link_selector: "a".to_string(),
            time_selector: ".w-annc__postdate".to_string(),
            tags_selector: Some(".tags".to_string()),
        }
    }
}

/// 子內容選取器設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct SubSelectionConfig {
    /// 文章全文選取器
    pub full_content: String,

    /// 作者或單位選取器
    pub author_unit: String,

    /// 附件列表選取器
    pub attachments: String,

    /// 附件屬性名稱，如 href、src
    pub attachment_attr: String,
}

impl Default for SubSelectionConfig {
    fn default() -> Self {
        Self {
            full_content: ".article-content-box".to_string(),
            author_unit: ".info-dept".to_string(),
            attachments: ".file-download-list a".to_string(),
            attachment_attr: "href".to_string(),
        }
    }
}

/// 通知設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct NotificationConfig {
    /// Discord 通知設定
    pub discord: DiscordNotificationConfig,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            discord: DiscordNotificationConfig::default(),
        }
    }
}

/// Discord 推播設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct DiscordNotificationConfig {
    /// Discord Webhook URL
    pub webhook_url: String,

    /// Webhook 名稱模板
    pub webhook_name_template: String,

    /// Webhook 頭像 URL
    pub webhook_avatar_url: String,

    /// 發送訊息的頻道 ID
    pub channel_id: u64,

    /// Embed 標題
    pub embed_title: String,

    /// Embed 顏色（十進位）
    pub embed_color: u32,

    /// Embed 欄位設定
    pub fields: Vec<DiscordFieldConfig>,

    /// 是否附帶附件
    pub include_attachments: bool,
}

impl Default for DiscordNotificationConfig {
    fn default() -> Self {
        Self {
            webhook_url: "https://discord.com/api/webhooks/...".to_string(),
            webhook_name_template: "MyCrawlerBot".to_string(),
            webhook_avatar_url: "".to_string(),
            channel_id: 123456789012345678,
            embed_title: "📌 {title}".to_string(),
            embed_color: 3447003,
            fields: vec![
                DiscordFieldConfig {
                    name: "發佈單位".to_string(),
                    value: "{author_unit}".to_string(),
                    inline: true,
                },
                DiscordFieldConfig {
                    name: "類別".to_string(),
                    value: "{category}".to_string(),
                    inline: true,
                },
                DiscordFieldConfig {
                    name: "公告連結".to_string(),
                    value: "[點我開啟]({url})".to_string(),
                    inline: true,
                },
                DiscordFieldConfig {
                    name: "摘要".to_string(),
                    value: "{summary}".to_string(),
                    inline: true,
                },
                DiscordFieldConfig {
                    name: "標籤".to_string(),
                    value: "{tags}".to_string(),
                    inline: false,
                },
            ],
            include_attachments: false,
        }
    }
}

/// Discord Embed 欄位設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct DiscordFieldConfig {
    /// 欄位名稱
    pub name: String,

    /// 欄位值
    pub value: String,

    /// 是否內聯顯示
    pub inline: bool,
}

/// LLM 相關設定
#[derive(Debug, Serialize, Deserialize, Clone, DocReader)]
pub struct CrawlerLlmConfig {
    /// 是否啟用 LLM 功能
    pub enabled: bool,

    /// LLM API URL
    pub api_url: String,

    /// 使用的模型名稱
    pub model: String,

    /// 存放 API Key 的環境變數名稱
    pub api_key_env: String,

    /// 知識庫列表，用於 LLM 的輔助參考
    pub knowledge_base: Vec<String>,
}

impl Default for CrawlerLlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: "https://api.example-llm.com".to_string(),
            model: "qwen2.5-14b".to_string(),
            api_key_env: "LLM_API_KEY".to_string(),
            knowledge_base: vec![
                "管理學院".to_string(),
                "資管系".to_string(),
                "校區".to_string(),
                "全校".to_string(),
                "本班".to_string(),
                "通識".to_string(),
                "必修課程".to_string(),
                "學分".to_string(),
                "宿舍".to_string(),
                "交換生".to_string(),
                "獎學金".to_string(),
            ],
        }
    }
}
