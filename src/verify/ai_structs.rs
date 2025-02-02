use serde::{Deserialize, Serialize};
use std::fmt;

use super::{deepseek::structs::AssistantMessageDeepSeek, openai::structs::AssistantMessage};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PromptType {
    Website,
    #[default]
    Code,
    FullReview,
}

impl fmt::Display for PromptType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let as_str = match self {
            PromptType::Website => "website_content",
            PromptType::Code => "code_content",
            PromptType::FullReview => "all_analysis_to_review",
        };
        write!(f, "{}", as_str)
    }
}

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
pub struct AiChatCompletion<T, K> {
    id: String,
    object: String,
    created: i64,
    model: String,
    pub choices: Vec<Choice<T>>,
    #[serde(default)]
    usage: K,
    #[serde(default)]
    system_fingerprint: Option<String>,
}

#[derive(Deserialize)]
pub struct Choice<T> {
    index: i64,
    pub message: T, // AssistantMessageDeepSeek
    #[serde(default)]
    logprobs: Option<()>,
    #[serde(default)]
    finish_reason: Option<String>,
}

/// The content we expect the model to produce is a JSON object with:
/// { "possible_scam": <true/false>, "reason": "<2~3 sentences>" }
#[derive(Deserialize, Clone, Debug)]
pub struct TokenCodeCheck {
    pub possible_scam: bool,
    pub reason: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
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

/// Error response
#[derive(Deserialize, Debug)]
pub struct AiErrorResponse {
    pub error: AiErrorDetail,
}

#[derive(Deserialize, Debug)]
pub struct AiErrorDetail {
    pub message: String,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

pub trait HasContent {
    fn get_content(&self) -> Option<String>;
}

impl HasContent for AssistantMessage {
    fn get_content(&self) -> Option<String> {
        self.content.clone()
    }
}

impl HasContent for AssistantMessageDeepSeek {
    fn get_content(&self) -> Option<String> {
        self.content.clone()
    }
}
