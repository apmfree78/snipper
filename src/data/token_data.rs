use crate::abi::erc20::ERC20;
use anyhow::Result;
use ethers::providers::{Provider, Ws};
use ethers::types::Address;
use futures::lock::Mutex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

use super::tokens::Erc20Token;

static TOKEN_HASH: Lazy<Arc<Mutex<HashMap<String, Erc20Token>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::<String, Erc20Token>::new())));

pub async fn get_and_save_erc20_by_token_address(
    token_address_str: &str,
    pool_address: &str,
    fee: u32,
    client: &Arc<Provider<Ws>>,
) -> Result<Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;

    // make sure token is not already in hashmap
    if tokens.contains_key(token_address_str) {
        let token = tokens.get(token_address_str).unwrap();
        return Ok(token.clone());
    }

    let token_address: Address = token_address_str.parse()?;
    let token_contract = ERC20::new(token_address, client.clone());

    // get basic toke data
    let symbol = token_contract.symbol().call().await?;
    let decimals = token_contract.decimals().call().await?;
    let name = token_contract.name().call().await?;

    let token = Erc20Token {
        name,
        symbol,
        decimals,
        fee,
        address: token_address_str.to_lowercase(),
        pool_address: pool_address.to_string(),
        ..Default::default()
    };

    tokens.insert(token.symbol.clone(), token.clone());
    tokens.insert(token.address.to_lowercase(), token.clone());

    Ok(token)
}

pub async fn get_tokens() -> HashMap<String, Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;

    tokens.clone()
}

pub async fn get_token(token_address: &str) -> Option<Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;

    if let Some(token) = tokens.get(token_address) {
        Some(token.clone())
    } else {
        None
    }
}

pub async fn remove_token(token_address: &str) -> Option<Erc20Token> {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;

    tokens.remove(token_address)
}

pub async fn update_token(updated_token: &Erc20Token) {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let mut tokens = token_data_hash.lock().await;

    tokens.insert(updated_token.address.to_lowercase(), updated_token.clone());
}
