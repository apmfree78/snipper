use serde::{Deserialize, Serialize};
/// ---------------------
///   Request Structures
/// ---------------------
#[derive(Serialize)]
pub struct ChatCompletionRequest {
    /// The model name (e.g. "gpt-3.5-turbo", "gpt-4").
    pub model: String,
    /// The sequence of messages for the chat.
    pub messages: Vec<MessageToSend>,
    /// Sampling temperature.
    pub temperature: f64,
    /// Max tokens to generate in the completion.
    pub max_tokens: usize,
    /// The cumulative probability at which to cut off sampling.
    pub top_p: f64,
}

/// Represents a single message in the conversation sent to the model.
#[derive(Serialize)]
pub struct MessageToSend {
    /// The role (system, user, or assistant).
    pub role: String,
    /// The content of the message.
    pub content: String,
}

/// ---------------------
///   Response Structures
/// ---------------------
/// This struct mirrors the entire JSON response OpenAI sends back.
#[derive(Deserialize)]
pub struct OpenAiChatCompletion {
    id: String,
    object: String,
    created: i64,
    model: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    usage: Usage,
    #[serde(default)]
    system_fingerprint: Option<String>,
}

#[derive(Deserialize)]
pub struct Choice {
    index: i64,
    pub message: AssistantMessage,
    #[serde(default)]
    logprobs: Option<()>,
    #[serde(default)]
    finish_reason: Option<String>,
}

/// We assume the assistant's message content is strictly valid JSON
/// matching our ScamCheck schema.
#[derive(Deserialize)]
pub struct AssistantMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub refusal: Option<()>,
}

/// The content we expect the model to produce is a JSON object with:
/// { "possible_scam": <true/false>, "reason": "<2~3 sentences>" }
#[derive(Deserialize, Clone, Debug)]
pub struct TokenCodeCheck {
    pub possible_scam: bool,
    pub reason: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TokenWebsiteCheck {
    pub possible_scam: bool,
    pub reason: String,
    pub summary: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TokenFinalAssessment {
    pub final_scam_assessment: bool,
    pub reason: String,
    pub could_legitimately_justify_suspicious_code: bool,
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

/// Error response
#[derive(Deserialize, Debug)]
pub struct OpenAiErrorResponse {
    pub error: OpenAiErrorDetail,
}

#[derive(Deserialize, Debug)]
pub struct OpenAiErrorDetail {
    pub message: String,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

pub fn get_openai_api_key() -> anyhow::Result<String> {
    let etherscan_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set in .env");

    Ok(etherscan_key)
}
