use crate::abi::erc20::ERC20;
use crate::events::PairCreatedEvent;
use crate::swap::anvil_validation::{
    validate_token_with_simulated_buy_sell, TokenLiquidity, TokenStatus,
};
use crate::swap::token_price::get_token_weth_total_supply;
use crate::utils::type_conversion::address_to_string;
use anyhow::Result;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use futures::lock::Mutex;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

use super::contracts::CONTRACT;
use super::tokens::Erc20Token;

static TOKEN_HASH: Lazy<Arc<Mutex<HashMap<String, Erc20Token>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::<String, Erc20Token>::new())));

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

    // make sure token is not already in hashmap
    if tokens.contains_key(&token_address_string) {
        let token = tokens.get(&token_address_string).unwrap();
        return Ok(Some(token.clone()));
    }

    let token_contract = ERC20::new(token_address, client.clone());

    // get basic toke data
    info!("getting basic token info...");
    let symbol = token_contract.symbol().call().await?;
    let decimals = token_contract.decimals().call().await?;
    let name = token_contract.name().call().await?;

    let token = Erc20Token {
        name,
        symbol,
        decimals,
        address: token_address,
        pair_address: pair_created_event.pair,
        is_token_0,
        ..Default::default()
    };

    tokens.insert(token_address_string, token.clone());

    Ok(Some(token))
}

pub async fn display_token_volume_stats() -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    println!("----------------------------------------------");
    println!("----------------TOKEN STATS------------------");
    println!("----------------------------------------------");
    for token in tokens.values() {
        token.display_token_portfolio_volume_interval()?;
    }

    Ok(())
}

pub async fn display_token_time_stats() -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    println!("----------------------------------------------");
    println!("----------------TOKEN STATS------------------");
    println!("----------------------------------------------");
    for token in tokens.values() {
        token.display_token_portfolio_time_interval()?;
    }

    Ok(())
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

pub async fn check_all_tokens_are_tradable(client: &Arc<Provider<Ws>>) -> anyhow::Result<()> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;

    for token in tokens.values_mut() {
        if !token.is_tradable {
            // check liquidity
            let total_supply = get_token_weth_total_supply(&token, client).await?;

            if total_supply > U256::from(0) {
                token.is_tradable = true;
                info!("{} is tradable", token.name);
            } else {
                info!("{} is not tradable", token.name);
            }
        }
    }

    Ok(())
}

pub async fn validate_tradable_tokens() -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    let mut handles = vec![];
    for token_ref in tokens.values() {
        let token = token_ref.clone();
        // SEPARATE THREAD FOR EACH TOKEN VALIDATION CHECK
        let handle = tokio::spawn(async move {
            let result: anyhow::Result<()> = async move {
                if token.is_tradable && !token.is_validated && !token.is_validating {
                    set_token_to_validating(&token).await;

                    let token_status =
                        validate_token_with_simulated_buy_sell(&token, TokenLiquidity::HasEnough)
                            .await?;
                    if token_status == TokenStatus::Legit {
                        info!("{} is validated!", token.name);
                        set_token_to_validated(&token).await;
                    } else {
                        let scam_token = remove_token(token.address).await;
                        let scam_token = scam_token.unwrap();
                        warn!("removed {}", scam_token.symbol);
                    }
                }
                Ok(())
            }
            .await;

            if let Err(e) = result {
                error!("Error running validation thread: {:#}", e);
            }
        });

        handles.push(handle);
    }

    Ok(())
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

pub async fn update_token(updated_token: &Erc20Token) {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address = updated_token.lowercase_address();
    tokens.insert(token_address, updated_token.clone());
}

pub async fn update_token_gas_cost(token_address: Address, gas_cost: U256) {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address_string = address_to_string(token_address).to_lowercase();

    match tokens.get_mut(&token_address_string) {
        Some(token) => {
            token.tx_gas_cost += gas_cost;
        }
        None => {
            error!(
                "{} is not in token hash, cannot update.",
                token_address_string
            );
        }
    }
}

pub async fn set_token_to_validated(token: &Erc20Token) {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address_string = token.lowercase_address();

    match tokens.get_mut(&token_address_string) {
        Some(token) => {
            token.is_validating = false;
            token.is_validated = true;
        }
        None => {
            error!(
                "{} is not in token hash, cannot update.",
                token_address_string
            );
        }
    }
}

pub async fn set_token_to_sold(token: &Erc20Token) {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address_string = token.lowercase_address();

    match tokens.get_mut(&token_address_string) {
        Some(token) => {
            token.is_sold = true;
        }
        None => {
            error!(
                "{} is not in token hash, cannot update.",
                token_address_string
            );
        }
    }
}

pub async fn set_token_to_validating(token: &Erc20Token) {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address_string = token.lowercase_address();

    match tokens.get_mut(&token_address_string) {
        Some(token) => {
            token.is_validating = true;
        }
        None => {
            error!(
                "{} is not in token hash, cannot update.",
                token_address_string
            );
        }
    }
}

pub async fn get_number_of_tokens() -> usize {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;

    tokens.len()
}
