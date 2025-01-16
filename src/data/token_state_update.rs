use crate::abi::erc20::ERC20;
use crate::app_config::BLACKLIST;
use crate::data::contracts;
use crate::events::PairCreatedEvent;
use crate::utils::type_conversion::address_to_string;
use crate::verify::etherscan_api::get_source_code;
use anyhow::Result;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use log::{error, info, warn};
use std::sync::Arc;

use super::contracts::CONTRACT;
use super::token_data::TOKEN_HASH;
use super::tokens::{Erc20Token, TokenLiquidity, TokenState};

impl Erc20Token {
    pub async fn update_state(&self) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address = self.lowercase_address();
        tokens.insert(token_address, self.clone());
    }

    pub async fn set_state_to_(&self, state: TokenState) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => token.state = state,
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn update_post_purchase(&self, amount_bought: U256, time_of_purchase: u32) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => {
                token.is_tradable = true;
                token.amount_bought = amount_bought;
                token.time_of_purchase = time_of_purchase;
                token.state = TokenState::Bought;
            }
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn update_post_sale(&self, eth_recieved_at_sale: U256) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => {
                token.eth_recieved_at_sale = eth_recieved_at_sale;
                token.state = TokenState::Sold;
            }
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn update_post_time_sale(&self, amount_sold: U256, time_index: usize) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => {
                let mut current_amounts_sold = token.amount_sold_at_time.clone();
                let mut is_sold_at_time = token.is_sold_at_time.clone();

                if !is_sold_at_time[time_index] {
                    current_amounts_sold[time_index] = amount_sold;
                    is_sold_at_time[time_index] = true;

                    token.amount_sold_at_time = current_amounts_sold;
                    token.is_sold_at_time = is_sold_at_time;
                    token.state = TokenState::Bought;
                }
            }
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn set_liquidity_to_(&self, liquidity: TokenLiquidity) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => token.liquidity = liquidity,
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }

    pub async fn set_to_tradable_plus_update_liquidity(&mut self, liquidity: &TokenLiquidity) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => {
                token.is_tradable = true;
                token.liquidity = liquidity.clone();
                self.liquidity = liquidity.clone();
                println!("{} token liquidity set to {}", token.name, token.liquidity);
            }
            None => {
                error!(
                    "{} is not in token hash, cannot update.",
                    token_address_string
                );
            }
        }
    }
}

pub async fn remove_token(token_address: Address) -> Option<Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address_string = address_to_string(token_address).to_lowercase();

    match tokens.get(&token_address_string) {
        Some(_) => tokens.remove(&token_address_string),
        None => {
            warn!("token does not exist");
            None
        }
    }
}

pub async fn get_and_save_erc20_by_token_address(
    pair_created_event: &PairCreatedEvent,
    client: &Arc<Provider<Ws>>,
) -> Result<Option<Erc20Token>> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;

    // find address of new token
    let (token_address, is_token_0) = if weth_address == pair_created_event.token0 {
        (pair_created_event.token1, false)
    } else if weth_address == pair_created_event.token1 {
        (pair_created_event.token0, true)
    } else {
        warn!("not weth pair, skipping");
        return Ok(None);
    };

    let token_address_string = address_to_string(token_address).to_lowercase();

    // get solidity contract
    let contract_code = get_source_code(&token_address_string).await?;

    if contract_code.is_empty() {
        warn!("source code not avaliable, skipping");
        return Ok(None);
    }

    // make sure token is not already in hashmap
    if tokens.contains_key(&token_address_string) {
        let token = tokens.get(&token_address_string).unwrap();
        return Ok(Some(token.clone()));
    }

    let token_contract = ERC20::new(token_address, client.clone());

    // get basic toke data
    // info!("getting basic token info...");
    let symbol = token_contract.symbol().call().await?;
    let decimals = token_contract.decimals().call().await?;
    let name = token_contract.name().call().await?;

    if BLACKLIST.contains(&symbol.as_str()) {
        println!("blocked blacklisted token {}", name);
        return Ok(None);
    }

    info!("new token: {} ({}) detected!", name, symbol);

    let token = Erc20Token {
        name,
        symbol,
        decimals,
        address: token_address,
        source_code: contract_code,
        pair_address: pair_created_event.pair,
        is_token_0,
        ..Default::default()
    };

    tokens.insert(token_address_string, token.clone());

    Ok(Some(token))
}
