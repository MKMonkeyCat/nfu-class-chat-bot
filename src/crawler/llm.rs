use chrono::Utc;
use config::CrawlerLlmConfig;
use reqwest::Client;
use serde_json::json;

use super::types::{CrawledPost, LlmResult};

pub(crate) async fn call_llm(
    reqwest: &Client,
    llm_cfg: &CrawlerLlmConfig,
    post: &CrawledPost,
) -> Result<LlmResult, String> {
    let api_key = std::env::var(&llm_cfg.api_key_env)
        .map_err(|_| format!("missing env: {}", llm_cfg.api_key_env))?;

    let today_str = Utc::now().format("%Y-%m-%d").to_string();
    let prompt = build_prompt(
        &format!(
            "來源: {}\n標題: {}\n時間: {}\n連結: {}\n內容:\n{}",
            post.source_name, post.title, post.time, post.url, post.content
        ),
        &today_str,
        &llm_cfg.knowledge_base,
    );

    let payload = json!({
        "model": llm_cfg.model,
        "messages": [{ "role": "user", "content": prompt }],
        "temperature": 0.1,
        "response_format": { "type": "json_object" }
    });

    let response = reqwest
        .post(&llm_cfg.api_url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("llm request failed: {err}"))?;

    let response = response
        .error_for_status()
        .map_err(|err| format!("llm status failed: {err}"))?;

    let json_value: serde_json::Value = response
        .json()
        .await
        .map_err(|err| format!("invalid llm response json: {err}"))?;

    let content = json_value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| "llm response missing choices[0].message.content".to_string())?;

    serde_json::from_str::<LlmResult>(content)
        .map_err(|err| format!("llm content is not valid json object: {err}"))
}

fn build_prompt(user_input: &str, today_str: &str, kb: &[String]) -> String {
    let kb_context = kb.join("、");
    format!(
        r#"# ROLE
你是校園資訊架構師。你的任務是將公告內容精煉為結構化 JSON，並提供易讀的深度分析。

# CONTEXT
- 基準日期：{today_str}
- 關注範疇（知識庫）：{kb_context}

# RULES
1. **日期標準化**：所有日期轉為 "YYYY-MM-DD"。
2. **智慧判定 (is_relevant)**：僅當公告主旨或主要對象與知識庫直接相關時為 true。
3. **靈活解析 (analysis)**：
	 - 此欄位為「單一字串」。請使用 Markdown 換行與條列符號排版。
	 - **不要寫死結構**：請根據公告類型自動整理。例如：
	 - 若為「獎助學金」：整理對象、文件、流程。
	 - 若為「行政/停水/停電」：整理受影響範圍、時間、配套措施。
	 - 若為「講座/活動」：整理報名方式、活動地點、講者資訊。
	 - 若為「課程/選課」：整理適用對象、重要日期、流程說明。
	 - 若為「其他」：請靈活分析，確保重點清晰。
	 - 請詳細分析公告內容，並將重點以條列式呈現，確保資訊清晰且易於理解，但請確保無冗言贅字，僅保留具備「行動價值」的資訊。
4. **格式要求**：只輸出 JSON。
5. **禁止輸出**：請勿於非 "analysis_cn" 欄位輸出簡體，使用繁體中文。

# OUTPUT FORMAT
{{
	"is_relevant": true,
	"title": "公告標題",
	"summary": "一句話極簡概述",
	"analysis": "根據內容自動整理的條列式重點字串",
    "analysis_cn": "根據內容自動整理的條列式重點字串（簡體中文），該文字用於後續計算 SimHash",
	"calendar": [ {{ "date": "YYYY-MM-DD", "event": "事件名稱" }} ],
	"tags": ["關鍵字"]
}}

# DATA
<<<DATA
{user_input}
DATA>>>
"#
    )
}
