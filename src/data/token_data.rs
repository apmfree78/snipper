use crate::utils::tx::amount_of_token_to_purchase;
use crate::utils::type_conversion::address_to_string;
use ethers::types::{Address, U256};
use futures::lock::Mutex;
use log::error;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

use super::tokens::{Erc20Token, TokenState};

pub static TOKEN_HASH: Lazy<Arc<Mutex<HashMap<String, Erc20Token>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::<String, Erc20Token>::new())));

pub async fn token_count_by_state(state: TokenState) -> u32 {
    let tokens = get_tokens().await;

    tokens
        .into_values()
        .filter(|token| token.state == state)
        .count() as u32
}

pub async fn total_token_sales_revenue() -> U256 {
    let tokens = get_tokens().await;

    tokens
        .into_values()
        .map(|token| token.eth_recieved_at_sale)
        .fold(U256::zero(), |acc, x| acc.saturating_add(x))
}

pub async fn total_token_spend() -> anyhow::Result<U256> {
    let amount = amount_of_token_to_purchase()?;

    let token_count = token_count_by_state(TokenState::Bought).await;

    let total_spend = amount * U256::from(token_count);

    Ok(total_spend)
}

pub async fn total_token_gas_cost() -> U256 {
    let tokens = get_tokens().await;

    let mut total_gas = U256::zero();

    for token in tokens.values() {
        total_gas += token.tx_gas_cost;
    }

    total_gas
}

pub async fn get_tokens() -> HashMap<String, Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;

    tokens.clone()
}

pub async fn get_token(token_address: Address) -> Option<Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;
    let token_address_string = address_to_string(token_address).to_lowercase();

    if let Some(token) = tokens.get(&token_address_string) {
        Some(token.clone())
    } else {
        None
    }
}

impl Erc20Token {
    pub async fn is_sold_at_time_(&self, time_index: usize) -> bool {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let tokens = token_data_hash.lock().await;
        let token_address_string = address_to_string(self.address).to_lowercase();

        match tokens.get(&token_address_string) {
            Some(token) => token.is_sold_at_time[time_index],
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
                false
            }
        }
    }
}

pub async fn is_token_tradable(token_address: Address) -> bool {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;
    let token_address_string = address_to_string(token_address).to_lowercase();

    match tokens.get(&token_address_string) {
        Some(token) => token.is_tradable,
        None => {
            error!(
                "{} is not in token hash, cannot update.",
                token_address_string
            );
            false
        }
    }
}

pub async fn get_number_of_tokens() -> usize {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;

    tokens.len()
}
