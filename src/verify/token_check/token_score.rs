use serde::Deserialize;

use crate::{
    app_config::{
        FINAL_DETERMINATION_PROMPT_UPDATED, LIQUIDITY_PERCENTAGE_LOCKED,
        TOKEN_HOLDER_THRESHOLD_PERCENTAGE, VERY_LOW_LIQUIDITY_THRESHOLD,
    },
    verify::{
        ai_structs::{PromptType, TokenFinalAssessment},
        ai_submission::{chat_submission, AIChat, AIModel},
    },
};

use super::token_checklist::TokenCheckList;

// token will get a score based on TokenCheckList
#[derive(Debug)]
pub enum TokenScore {
    Legit = 4,
    LikelyLegit = 3,
    Iffy = 2,
    LikelyScam = 1,
    Scam = 0,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TokenScoreAssessment {
    pub token_score: String,
    pub reason: String,
}

/// Return token reputation score based on LLM evaluation
pub async fn get_token_score_with_ai(
    token_checklist: TokenCheckList,
    ai_model: &AIModel,
) -> anyhow::Result<Option<TokenScoreAssessment>> {
    let token_checklist = format!("{:#?}", token_checklist);

    let openai_chat = AIChat {
        prompt_instructions: FINAL_DETERMINATION_PROMPT_UPDATED.to_string(),
        ai_persona: "You are a solidity security expert and expert token investigator.".to_string(),
        prompt_content_to_review: token_checklist,
        prompt_type: PromptType::FullReview,
    };

    let token_final_score = chat_submission::<TokenScoreAssessment>(openai_chat, ai_model).await?;

    Ok(token_final_score)
}

/// Return token reputation score based on rules based approach
pub fn get_token_score_with_rules_based_approch(token_checklist: TokenCheckList) -> TokenScore {
    // check if token passed simulation
    match token_checklist.is_token_sellable {
        Some(false) => return TokenScore::Scam,
        None | Some(true) => {}
    }

    // check if total scam
    if token_checklist.possible_scam && !token_checklist.could_legitimately_justify_suspicious_code
    {
        return TokenScore::Scam;
    }

    // check that at least a high percetange ( typically 90 to 95%) of liquidity is locked or
    // burned
    let enough_liquidity_is_locked_or_burned =
        match token_checklist.percentage_liquidity_locked_or_burned {
            Some(percentage_locked_or_burned) => {
                percentage_locked_or_burned > LIQUIDITY_PERCENTAGE_LOCKED
            }
            None => false, // if could not calculate assume it false
        };

    // check that liquidity pool has enough liquidity , low liquidity usually indicates its a scam
    let enough_liquidity = token_checklist.liquidity_in_wei > VERY_LOW_LIQUIDITY_THRESHOLD;

    // check top token holder only holdes small percentage of tokens
    let top_token_holder_check =
        token_checklist.top_holder_percentage_tokens_held < TOKEN_HOLDER_THRESHOLD_PERCENTAGE;

    // check contract creator wallet only holdes small percentage of tokens
    // let creator_token_holdings_check =
    //     token_checklist.creator_percentage_tokens_held < TOKEN_HOLDER_THRESHOLD_PERCENTAGE;

    // if token is solidity code is clean
    if !token_checklist.possible_scam {
        if enough_liquidity_is_locked_or_burned && top_token_holder_check
        // && creator_token_holdings_check
        {
            if enough_liquidity {
                return TokenScore::Legit;
            } else {
                return TokenScore::LikelyLegit;
            }
        } else if enough_liquidity_is_locked_or_burned {
            if enough_liquidity {
                return TokenScore::Iffy;
            } else {
                return TokenScore::LikelyScam;
            }
        } else {
            if enough_liquidity {
                return TokenScore::LikelyScam;
            } else {
                return TokenScore::Scam;
            }
        }
    }

    if token_checklist.possible_scam && token_checklist.could_legitimately_justify_suspicious_code {
        if enough_liquidity {
            if enough_liquidity_is_locked_or_burned && top_token_holder_check
            // && creator_token_holdings_check
            {
                if token_checklist.has_website && token_checklist.has_twitter_or_discord {
                    return TokenScore::LikelyLegit;
                } else if token_checklist.has_website {
                    return TokenScore::Iffy;
                } else {
                    return TokenScore::LikelyScam;
                }
            } else if enough_liquidity_is_locked_or_burned {
                if token_checklist.has_website && token_checklist.has_twitter_or_discord {
                    return TokenScore::Iffy;
                } else if token_checklist.has_website {
                    return TokenScore::LikelyScam;
                } else {
                    return TokenScore::Scam;
                }
            } else {
                return TokenScore::Scam;
            }
        } else {
            return TokenScore::Scam;
        }
    }

    TokenScore::Scam
}
