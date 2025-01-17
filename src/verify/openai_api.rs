use anyhow::anyhow;
use log::warn;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::data::token_state_update::does_contact_exceed_size_limit;

/// ---------------------
///   Request Structures
/// ---------------------
#[derive(Serialize)]
struct ChatCompletionRequest {
    /// The model name (e.g. "gpt-3.5-turbo", "gpt-4").
    model: String,
    /// The sequence of messages for the chat.
    messages: Vec<MessageToSend>,
    /// Sampling temperature.
    temperature: f64,
    /// Max tokens to generate in the completion.
    max_tokens: usize,
    /// The cumulative probability at which to cut off sampling.
    top_p: f64,
}

/// Represents a single message in the conversation sent to the model.
#[derive(Serialize)]
struct MessageToSend {
    /// The role (system, user, or assistant).
    role: String,
    /// The content of the message.
    content: String,
}

/// ---------------------
///   Response Structures
/// ---------------------
/// This struct mirrors the entire JSON response OpenAI sends back.
#[derive(Deserialize, Debug)]
struct OpenAiChatCompletion {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default)]
    pub system_fingerprint: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    pub index: i64,
    pub message: AssistantMessage,
    #[serde(default)]
    pub logprobs: Option<()>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// We assume the assistant's message content is strictly valid JSON
/// matching our ScamCheck schema.
#[derive(Deserialize, Debug)]
struct AssistantMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub refusal: Option<()>,
}

/// The content we expect the model to produce is a JSON object with:
/// { "possible_scam": <true/false>, "reason": "<2~3 sentences>" }
#[derive(Deserialize, Clone, Debug)]
pub struct TokenAudit {
    pub possible_scam: bool,
    pub reason: String,
}

/// Usage object, as returned by OpenAI for cost analysis.
#[derive(Deserialize, Debug, Default)]
struct Usage {
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
struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: i64,
}

#[derive(Deserialize, Debug, Default)]
struct CompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: i64,
    #[serde(default)]
    pub accepted_prediction_tokens: i64,
    #[serde(default)]
    pub rejected_prediction_tokens: i64,
}

/// Error response
#[derive(Deserialize, Debug)]
struct OpenAiErrorResponse {
    error: OpenAiErrorDetail,
}
#[derive(Deserialize, Debug)]
struct OpenAiErrorDetail {
    message: String,
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

pub async fn audit_token_contract(source_code: String) -> anyhow::Result<Option<TokenAudit>> {
    // check contract size
    if source_code.is_empty() || does_contact_exceed_size_limit(&source_code) {
        warn!("contract is either empty or is too large");
        return Ok(None);
    }

    // Get your OpenAI API key from env
    let openai_api_key = get_openai_api_key()?;
    let client = Client::new();

    // Typically: store your big user prompt in a separate variable
    let user_prompt = r#"You are a expert Solidity security reviewer. I will provide you with an ERC‑20 contract source code. You need to check whether this contract has any signs of being a rug pull, honeypot, or other scam.

Pay special attention to:
1. The transfer function or `_transfer` logic (any hidden conditions or blacklists).
2. Ownership methods (`Ownable`, `renounceOwnership`, etc.) and whether ownership is *actually* renounced—or if there is a hidden or alternate owner variable.
3. Any ability for the owner or privileged account to mint additional tokens.
4. Any external calls or “rescue tokens,” “withdraw,” or “removeLiquidity” methods that could drain user funds or liquidity.
5. Unusually high or dynamically modifiable fees that could be set to extreme values.
6. Proxy or upgradeable patterns that could hide malicious updates later.
7. Any hidden or custom logic that prevents selling or imposes heavy taxes on sellers.
8. Disregard any trust signals such as “renounced ownership” or “burned liquidity” unless it is clear there is *no* backdoor enabling the developer to regain control or drain liquidity.

After analyzing these points, respond **strictly** in the following JSON format (no additional text). The `reason` should not exceed 2 to 3 sentences:

{ "possible_scam": <true_or_false>, "reason": "<2_or_3_sentences_describing_rationale>" }


Please only produce valid JSON—no code fencing or extra explanation. Provide a Boolean for `possible_scam`.

FOLLOWED BY the solidity source code which will be in a String called \"source_code\"."#;

    // Combine prompt + source code in one user content
    let user_content = format!("{}\n\nsource_code:\n{}", user_prompt, source_code);

    // Build the request body as a typed struct
    let request_body = ChatCompletionRequest {
        model: "gpt-4o".to_string(), // Or "gpt-4" / "gpt-4o-2024-08-06"
        messages: vec![
            MessageToSend {
                role: "system".to_string(),
                content: "You are a solidity security expert and token analyst.".to_string(),
            },
            MessageToSend {
                role: "user".to_string(),
                content: user_content,
            },
        ],
        temperature: 0.3,
        max_tokens: 30_000,
        top_p: 1.0,
    };

    // POST to OpenAI, automatically JSON-encode request_body via serde
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(openai_api_key)
        .json(&request_body)
        .send()
        .await?;

    // If status != 200, parse an error
    if !response.status().is_success() {
        let err_json: OpenAiErrorResponse = response.json().await?;
        return Err(anyhow!(
            "OpenAI Error: {} (type={:?}, code={:?})",
            err_json.error.message,
            err_json.error.r#type,
            err_json.error.code
        ));
    }

    let resp: OpenAiChatCompletion = response.json().await?;

    println!("response => {:#?}", resp);

    // Grab the first choice
    let first_choice = resp
        .choices
        .get(0)
        .ok_or_else(|| anyhow!("No choices returned from OpenAI"))?;

    // The `content` is a string that we expect to contain JSON
    let audit_str = first_choice
        .message
        .content
        .as_ref()
        .ok_or_else(|| anyhow!("No 'content' field in the assistant's message"))?;

    // Try to parse it as JSON
    let token_audit: TokenAudit = match serde_json::from_str(audit_str) {
        Ok(parsed) => parsed,
        Err(e) => {
            // The model's output wasn't valid JSON. Let's handle that gracefully:
            println!(
                "Assistant didn't return expected JSON. Received:\n{}\n\nError was: {}",
                audit_str, e
            );
            return Ok(None);
        }
    };

    Ok(Some(token_audit.clone()))
}

fn get_openai_api_key() -> anyhow::Result<String> {
    let etherscan_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set in .env");

    Ok(etherscan_key)
}
