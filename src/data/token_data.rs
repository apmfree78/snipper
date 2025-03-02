use crate::abi::erc20::ERC20;
use crate::swap::token_price::get_token_weth_liquidity;
use crate::uniswap_v3_events::PoolCreatedEvent;
use crate::utils::type_conversion::address_to_string;
use anyhow::Result;
use ethers::providers::{Provider, Ws};
use ethers::types::Address;
use futures::lock::Mutex;
use log::warn;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

use super::contracts::CONTRACT;
use super::tokens::Erc20Token;

static TOKEN_HASH: Lazy<Arc<Mutex<HashMap<String, Erc20Token>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::<String, Erc20Token>::new())));

pub async fn get_and_save_erc20_by_token_address(
    pool_created_event: &PoolCreatedEvent,
    client: &Arc<Provider<Ws>>,
) -> Result<Option<Erc20Token>> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;

    // find address of new token
    let (token_address, is_token_0) = if weth_address == pool_created_event.token0 {
        (pool_created_event.token1, false)
    } else if weth_address == pool_created_event.token1 {
        (pool_created_event.token0, true)
    } else {
        warn!("not weth pool, skipping");
        return Ok(None);
    };

    // TODO - VALIDATE TOKEN HERE - IF SCAM exit out

    let token_address_string = address_to_string(token_address).to_lowercase();

    // make sure token is not already in hashmap
    if tokens.contains_key(&token_address_string) {
        let token = tokens.get(&token_address_string).unwrap();
        return Ok(Some(token.clone()));
    }

    let token_contract = ERC20::new(token_address, client.clone());

    // get basic toke data
    let symbol = token_contract.symbol().call().await?;
    let decimals = token_contract.decimals().call().await?;
    let name = token_contract.name().call().await?;

    let token = Erc20Token {
        name,
        symbol,
        decimals,
        fee: pool_created_event.fee,
        address: token_address,
        pool_address: pool_created_event.pool,
        is_token_0,
        ..Default::default()
    };

    tokens.insert(token_address_string, token.clone());

    Ok(Some(token))
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

pub async fn check_all_tokens_and_update_if_are_tradable(
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<()> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;

    for token in tokens.values_mut() {
        if !token.is_tradable {
            // check liquidity
            let token_liquidity = get_token_weth_liquidity(&token, client).await?;

            if token_liquidity > 0 {
                token.is_tradable = true;
            }
        }
    }

    Ok(())
}

pub async fn remove_token(token_address: Address) -> Option<Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address_string = address_to_string(token_address).to_lowercase();

    tokens.remove(&token_address_string)
}

pub async fn is_token_tradable(token_address: Address) -> bool {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;
    let token_address_string = address_to_string(token_address).to_lowercase();

    let token = tokens.get(&token_address_string).unwrap();

    token.is_tradable
}

pub async fn update_token(updated_token: &Erc20Token) {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;
    let token_address = address_to_string(updated_token.address).to_lowercase();

    tokens.insert(token_address, updated_token.clone());
}

pub async fn get_number_of_tokens() -> usize {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;

    tokens.len()
}
