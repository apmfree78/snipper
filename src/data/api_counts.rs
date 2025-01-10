use super::token_data::TOKEN_HASH;
use super::tokens::Erc20Token;
use log::error;
use std::sync::Arc;

impl Erc20Token {
    pub async fn purchase_attempt_count(&self) -> u8 {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get(&token_address_string) {
            Some(token) => token.purchase_attempts,
            None => {
                error!(
                    "{} is not in token hash, cannot read.",
                    token_address_string
                );
                0u8
            }
        }
    }

    pub async fn increment_purchase_attempts(&self) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => token.purchase_attempts += 1,
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn sell_attempt_count(&self) -> u8 {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get(&token_address_string) {
            Some(token) => token.sell_attempts,
            None => {
                error!(
                    "{} is not in token hash, cannot read.",
                    token_address_string
                );
                0u8
            }
        }
    }

    pub async fn increment_sell_attempts(&self) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => token.sell_attempts += 1,
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn increment_honeypot_checks(&self) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => token.honeypot_checks += 1,
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn honeypot_check_count(&self) -> u8 {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get(&token_address_string) {
            Some(token) => token.honeypot_checks,
            None => {
                error!(
                    "{} is not in token hash, cannot read.",
                    token_address_string
                );
                0u8
            }
        }
    }

    pub async fn increment_graphql_checks(&self) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => token.graphql_checks += 1,
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn graphql_check_count(&self) -> u8 {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get(&token_address_string) {
            Some(token) => token.graphql_checks,
            None => {
                error!(
                    "{} is not in token hash, cannot read.",
                    token_address_string
                );
                0u8
            }
        }
    }
}
