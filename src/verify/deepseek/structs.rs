use serde::{Deserialize, Serialize};
/// ---------------------
///   Response Structures
/// ---------------------
#[derive(Deserialize)]
pub struct AssistantMessageDeepSeek {
    pub role: String,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Vec<()>,
}

/// Usage object, as returned by OpenAI for cost analysis.
#[derive(Deserialize, Debug, Default)]
pub struct UsageDeepSeek {
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
    #[serde(default)]
    pub prompt_cache_hit_tokens: i64,
    #[serde(default)]
    pub prompt_cache_miss_tokens: i64,
    #[serde(default)]
    pub completion_tokens_details: CompletionTokensDetailsDeepSeek,
}

#[derive(Deserialize, Debug, Default)]
pub struct CompletionTokensDetailsDeepSeek {
    #[serde(default)]
    pub reasoning_tokens: i64,
}

pub fn get_deepseek_api_key() -> anyhow::Result<String> {
    let etherscan_key =
        std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY is not set in .env");

    Ok(etherscan_key)
}
