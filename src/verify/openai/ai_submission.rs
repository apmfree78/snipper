use anyhow::anyhow;
use log::warn;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::fmt;

use crate::{
    app_config::{CODE_CHECK_PROMPT, FINAL_DETERMINATION_PROMPT, WEBSITE_CHECK_PROMPT},
    data::tokens::Erc20Token,
};

use super::structs::{
    get_openai_api_key, ChatCompletionRequest, MessageToSend, OpenAiChatCompletion,
    OpenAiErrorResponse, TokenCodeCheck, TokenWebsiteCheck,
};

#[derive(Clone, Debug, Default)]
pub struct OpenAIChat {
    pub prompt_instructions: String,
    pub ai_persona: String,
    pub prompt_content_to_review: String, // solidity code , website scraped data
    pub prompt_type: PromptType,
}

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

pub async fn full_token_review_with_ai(
    code_analysis: TokenCodeCheck,
    website_analysis: TokenWebsiteCheck,
    token: &Erc20Token,
) -> anyhow::Result<Option<TokenCodeCheck>> {
    let analysis_to_review = format!(
        "token_code_analysis:/n{:#?}/nwebsite_review:/n{:#?}/n{}",
        code_analysis,
        website_analysis,
        get_additional_context_sentense(token)
    );

    let website_openai_chat = OpenAIChat {
        prompt_instructions: FINAL_DETERMINATION_PROMPT.to_string(),
        ai_persona: "You are a solidity security expert and expert token investigator.".to_string(),
        prompt_content_to_review: analysis_to_review,
        prompt_type: PromptType::FullReview,
    };

    let code_check = openai_chat_submission::<TokenCodeCheck>(website_openai_chat).await?;

    Ok(code_check)
}
pub async fn check_code_with_ai(code: String) -> anyhow::Result<Option<TokenCodeCheck>> {
    let website_openai_chat = OpenAIChat {
        prompt_instructions: CODE_CHECK_PROMPT.to_string(),
        ai_persona: "You are a solidity security expert and token analyst.".to_string(),
        prompt_content_to_review: code,
        prompt_type: PromptType::Code,
    };

    let code_check = openai_chat_submission::<TokenCodeCheck>(website_openai_chat).await?;

    Ok(code_check)
}

pub async fn check_website_with_ai(
    website_content: String,
) -> anyhow::Result<Option<TokenWebsiteCheck>> {
    let website_openai_chat = OpenAIChat {
        prompt_instructions: WEBSITE_CHECK_PROMPT.to_string(),
        ai_persona:"You are an expert crypto investigator specializing in evaluating crypto website credibility.".to_string(),
        prompt_content_to_review:website_content,
        prompt_type: PromptType::Website
    };

    let website_check = openai_chat_submission::<TokenWebsiteCheck>(website_openai_chat).await?;

    Ok(website_check)
}

pub async fn openai_chat_submission<T>(openai_chat: OpenAIChat) -> anyhow::Result<Option<T>>
where
    T: DeserializeOwned,
{
    // check contract size
    if openai_chat.prompt_content_to_review.is_empty() {
        warn!("no {}", openai_chat.prompt_type);
        return Ok(None);
    }

    // Get your OpenAI API key from env
    let openai_api_key = get_openai_api_key()?;
    let client = Client::new();

    // Combine prompt + source code in one user content
    let content = format!(
        "{}\n\n{}:\n{}",
        openai_chat.prompt_instructions,
        openai_chat.prompt_type,
        openai_chat.prompt_content_to_review
    );

    // Build the request body as a typed struct
    let request_body = ChatCompletionRequest {
        model: "gpt-4o".to_string(), // Or "gpt-4" / "gpt-4o-2024-08-06"
        messages: vec![
            MessageToSend {
                role: "system".to_string(),
                content: openai_chat.ai_persona, //"You are an expert crypto investigator specializing in evaluating crypto website credibility.".to_string(),
            },
            MessageToSend {
                role: "user".to_string(),
                content,
            },
        ],
        temperature: 0.3,
        max_tokens: 16_000,
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

    // println!("response => {:#?}", resp);

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
    let audit: T = match serde_json::from_str(audit_str) {
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

    Ok(Some(audit))
}

fn get_additional_context_sentense(token: &Erc20Token) -> String {
    let online_precense = &token.token_web_data;

    let addional_context = if !online_precense.website.is_empty()
        || !online_precense.twitter.is_empty()
        || !online_precense.discord.is_empty()
        || !online_precense.whitepaper.is_empty()
    {
        let mut context = "".to_string();

        context = if !online_precense.twitter.is_empty() {
            format!("/n twitter: {}/n", online_precense.twitter)
        } else {
            context
        };

        context = if !online_precense.website.is_empty() {
            format!("{} website: {}/n", context, online_precense.website)
        } else {
            context
        };

        context = if !online_precense.discord.is_empty() {
            format!("{} discord: {}/n", context, online_precense.discord)
        } else {
            context
        };

        context = if !online_precense.whitepaper.is_empty() {
            format!("{} whitepaper: {}/n", context, online_precense.whitepaper)
        } else {
            context
        };

        context = format!("For more context this token also has {}", context);

        context
    } else {
        "".to_string()
    };

    addional_context
}
