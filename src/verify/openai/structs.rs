use serde::Deserialize;
/// ---------------------
///   Response Structures
/// ---------------------
///// This struct mirrors the entire JSON response OpenAI sends back.
/// We assume the assistant's message content is strictly valid JSON
/// matching our ScamCheck schema.
#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub refusal: Option<()>,
}

/// Usage object, as returned by OpenAI for cost analysis.
#[derive(Deserialize, Debug, Default)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
    #[serde(default)]
    pub prompt_tokens_details: PromptTokensDetails,
    #[serde(default)]
    pub completion_tokens_details: CompletionTokensDetails,
}

#[derive(Deserialize, Debug, Default)]
pub struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: i64,
}

#[derive(Deserialize, Debug, Default)]
pub struct CompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: i64,
    #[serde(default)]
    pub accepted_prediction_tokens: i64,
    #[serde(default)]
    pub rejected_prediction_tokens: i64,
}

pub fn get_openai_api_key() -> anyhow::Result<String> {
    let etherscan_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set in .env");

    Ok(etherscan_key)
}
