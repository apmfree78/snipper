use log::warn;

use crate::{
    app_config::{CODE_CHECK_PROMPT, FINAL_DETERMINATION_PROMPT, WEBSITE_CHECK_PROMPT},
    data::tokens::Erc20Token,
};

use super::{
    ai_structs::TokenFinalAssessment,
    ai_submission::{
        check_code_with_ai, check_website_with_ai, full_token_review_with_ai, AIModel,
    },
};

impl Erc20Token {
    pub async fn ai_analysis(
        &self,
        ai_model: &AIModel,
    ) -> anyhow::Result<Option<TokenFinalAssessment>> {
        let code_check = check_code_with_ai(self.source_code.clone(), ai_model).await?;

        let code_check = match code_check {
            Some(check) => check,
            None => {
                warn!("could not perform code check!");
                return Ok(None);
            }
        };

        let website_url = self.token_web_data.website.clone();
        let website_content = self.token_web_data.scraped_web_content.clone();

        if !website_url.is_empty() && !website_content.is_empty() {
            //TODO - FIX must pass website content
            let website_check = check_website_with_ai(website_content, ai_model).await?;

            let website_check = match website_check {
                Some(check) => check,
                None => {
                    warn!("could not perform website check!");
                    return Ok(None);
                }
            };

            let full_analysis =
                full_token_review_with_ai(code_check, website_check, self, ai_model).await?;
            return Ok(full_analysis);
        } else {
            return Ok(Some(TokenFinalAssessment {
                final_scam_assessment: code_check.possible_scam,
                reason: code_check.reason,
                could_legitimately_justify_suspicious_code: false,
            }));
        }
    }
}
