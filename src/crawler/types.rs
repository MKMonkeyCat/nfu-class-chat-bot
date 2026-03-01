use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct CrawlerBasePost {
    pub(crate) id: String,
    pub(crate) category: String,
    pub(crate) source_name: String,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) time: String,
    pub(crate) tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CrawledPost {
    pub(crate) id: String,
    pub(crate) category: String,
    pub(crate) source_name: String,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) content: String,
    pub(crate) author_unit: String,
    pub(crate) attachments: Vec<String>,
    pub(crate) time: String,
    pub(crate) tags: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct LlmCalendarItem {
    #[serde(default)]
    pub(crate) date: String,
    #[serde(default)]
    pub(crate) event: String,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct LlmResult {
    #[serde(default)]
    pub(crate) is_relevant: bool,
    #[serde(default)]
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) analysis: String,
    #[serde(default)]
    pub(crate) analysis_cn: String,
    #[serde(default)]
    pub(crate) calendar: Vec<LlmCalendarItem>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
}
