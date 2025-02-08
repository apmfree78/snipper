use std::fmt::Debug;

use anyhow::anyhow;
use log::warn;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;

use crate::{
    app_config::{CODE_CHECK_PROMPT, FINAL_DETERMINATION_PROMPT, WEBSITE_CHECK_PROMPT},
    data::tokens::Erc20Token,
};

use super::{
    ai_structs::{
        AiChatCompletion, AiErrorResponse, ChatCompletionRequest, HasContent, MessageToSend,
        PromptType, TokenCodeCheck, TokenFinalAssessment, TokenWebsiteCheck,
    },
    deepseek::structs::{get_deepseek_api_key, AssistantMessageDeepSeek, UsageDeepSeek},
    openai::structs::{get_openai_api_key, AssistantMessage, Usage},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AIModel {
    DeepSeek,
    OpenAi,
}

#[derive(Clone, Debug, Default)]
pub struct AIChat {
    pub prompt_instructions: String,
    pub ai_persona: String,
    pub prompt_content_to_review: String, // solidity code , website scraped data
    pub prompt_type: PromptType,
}

pub async fn full_token_review_with_ai(
    code_analysis: TokenCodeCheck,
    website_analysis: TokenWebsiteCheck,
    token: &Erc20Token,
    ai_model: &AIModel,
) -> anyhow::Result<Option<TokenFinalAssessment>> {
    let additional_context = get_additional_context_sentense(token);

    println!("additonal context ==> {}", additional_context);
    let analysis_to_review = format!(
        "token_code_analysis:\n{:#?}\nwebsite_review:\n{:#?}\n{}",
        code_analysis, website_analysis, additional_context
    );

    let website_openai_chat = AIChat {
        prompt_instructions: FINAL_DETERMINATION_PROMPT.to_string(),
        ai_persona: "You are a solidity security expert and expert token investigator.".to_string(),
        prompt_content_to_review: analysis_to_review,
        prompt_type: PromptType::FullReview,
    };

    // println!("final prompt => {}", format!("{:#?}", website_openai_chat));

    let code_check = chat_submission::<TokenFinalAssessment>(website_openai_chat, ai_model).await?;

    Ok(code_check)
}

pub async fn check_code_with_ai(
    code: String,
    ai_model: &AIModel,
) -> anyhow::Result<Option<TokenCodeCheck>> {
    let website_openai_chat = AIChat {
        prompt_instructions: CODE_CHECK_PROMPT.to_string(),
        ai_persona: "You are a solidity security expert and token analyst.".to_string(),
        prompt_content_to_review: code,
        prompt_type: PromptType::Code,
    };

    let code_check = chat_submission::<TokenCodeCheck>(website_openai_chat, ai_model).await?;

    Ok(code_check)
}

pub async fn check_website_with_ai(
    website_content: String,
    ai_model: &AIModel,
) -> anyhow::Result<Option<TokenWebsiteCheck>> {
    let website_openai_chat = AIChat {
        prompt_instructions: WEBSITE_CHECK_PROMPT.to_string(),
        ai_persona:"You are an expert crypto investigator specializing in evaluating crypto website credibility.".to_string(),
        prompt_content_to_review:website_content,
        prompt_type: PromptType::Website
    };

    let website_check = chat_submission::<TokenWebsiteCheck>(website_openai_chat, ai_model).await?;

    Ok(website_check)
}

pub async fn chat_submission<T>(chat: AIChat, ai_model: &AIModel) -> anyhow::Result<Option<T>>
where
    T: DeserializeOwned,
{
    // check contract size
    if chat.prompt_content_to_review.is_empty() {
        warn!("no {}", chat.prompt_type);
        return Ok(None);
    }

    // Get your OpenAI API key from env
    let api_key = match ai_model {
        AIModel::OpenAi => get_openai_api_key()?,
        AIModel::DeepSeek => get_deepseek_api_key()?,
    };

    let client = Client::new();

    // Combine prompt + source code in one user content
    let content = format!(
        "{}\n\n{}:\n{}",
        chat.prompt_instructions, chat.prompt_type, chat.prompt_content_to_review
    );

    let (model, url, max_tokens) = match ai_model {
        AIModel::DeepSeek => (
            "deepseek-reasoner".to_string(),
            "https://api.deepseek.com",
            8_000,
        ),
        AIModel::OpenAi => ("gpt-4o".to_string(), "https://api.openai.com/v1", 16_000),
    };

    // Build the request body as a typed struct
    let request_body = ChatCompletionRequest {
        model, // Or "gpt-4" / "gpt-4o-2024-08-06"
        messages: vec![
            MessageToSend {
                role: "system".to_string(),
                content: chat.ai_persona, //"You are an expert crypto investigator specializing in evaluating crypto website credibility.".to_string(),
            },
            MessageToSend {
                role: "user".to_string(),
                content,
            },
        ],
        temperature: 0.3,
        max_tokens,
        top_p: 1.0,
    };

    let api_url = format!("{}/chat/completions", url);

    // POST to OpenAI, automatically JSON-encode request_body via serde
    let response = client
        .post(api_url)
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await?;

    // If status != 200, parse an error
    if !response.status().is_success() {
        let err_json: AiErrorResponse = response.json().await?;
        return Err(anyhow!(
            "OpenAI Error: {} (type={:?}, code={:?})",
            err_json.error.message,
            err_json.error.r#type,
            err_json.error.code
        ));
    }

    if *ai_model == AIModel::OpenAi {
        return process_response::<T, AssistantMessage, Usage>(response).await;
    } else {
        return process_response::<T, AssistantMessageDeepSeek, UsageDeepSeek>(response).await;
    }
}

async fn process_response<T, K, M>(response: Response) -> anyhow::Result<Option<T>>
where
    T: DeserializeOwned,
    K: DeserializeOwned + HasContent + Debug,
    M: DeserializeOwned + Default + Debug,
{
    let resp: AiChatCompletion<K, M> = response.json().await?;

    // Grab the first choice
    let first_choice = resp
        .choices
        .get(0)
        .ok_or_else(|| anyhow!("No choices returned from OpenAI"))?;

    // The `content` is a string that we expect to contain JSON
    let audit_string = first_choice
        .message
        .get_content()
        .ok_or_else(|| anyhow!("No 'content' field in the assistant's message"))?;

    // Try to parse it as JSON
    let audit: T = match serde_json::from_str(&audit_string) {
        Ok(parsed) => parsed,
        Err(e) => {
            // The model's output wasn't valid JSON. Let's handle that gracefully:
            println!(
                "Assistant didn't return expected JSON. Received:\n{}\n\nError was: {}",
                audit_string, e
            );
            return Ok(None);
        }
    };

    return Ok(Some(audit));
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
            format!("\n twitter: {}\n", online_precense.twitter)
        } else {
            context
        };

        context = if !online_precense.website.is_empty() {
            format!("{} website: {}\n", context, online_precense.website)
        } else {
            context
        };

        context = if !online_precense.discord.is_empty() {
            format!("{} discord: {}\n", context, online_precense.discord)
        } else {
            context
        };

        context = if !online_precense.whitepaper.is_empty() {
            format!("{} whitepaper: {}\n", context, online_precense.whitepaper)
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
