use crate::abi::erc20::ERC20;
use crate::events::PairCreatedEvent;
use crate::swap::anvil::validation::{TokenLiquidity, TokenStatus};
use crate::token_tx::time_intervals::TIME_ROUNDS;
use crate::utils::tx::get_token_sell_interval;
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
use super::tokens::{Erc20Token, TokenState};

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
    // info!("getting basic token info...");
    let symbol = token_contract.symbol().call().await?;
    let decimals = token_contract.decimals().call().await?;
    let name = token_contract.name().call().await?;
    info!("new token: {} ({}) detected!", name, symbol);

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
    let time_bought = get_token_sell_interval()?;

    let mut sum_profit_per_interval = [0.0; TIME_ROUNDS];
    let mut sum_roi_per_interval = [0.0; TIME_ROUNDS];
    let mut average_roi_per_interval = [0.0; TIME_ROUNDS];
    let mut tokens_sold_at_this_interval: [u32; TIME_ROUNDS] = [0; TIME_ROUNDS];
    println!("----------------------------------------------");
    println!("----------------TOKEN STATS------------------");
    println!("----------------------------------------------");
    for token in tokens.values() {
        let (profits, roi) = token.display_token_portfolio_time_interval()?;

        // Initialize or accumulate for profits
        if sum_profit_per_interval.is_empty() {
            // First token => just clone its entire vector
            sum_profit_per_interval = profits.clone();
        } else {
            // Add each element into the corresponding sum
            for (i, &p) in profits.iter().enumerate() {
                sum_profit_per_interval[i] += p;
                // if profit is exactly zero then token was not sold at this interval yet,
                // so do not count it when averaging out profit and roi
                tokens_sold_at_this_interval[i] += if p == 0.0 { 0 } else { 1 };
            }
        }

        // Similarly for roi
        if sum_roi_per_interval.is_empty() {
            sum_roi_per_interval = roi.clone();
        } else {
            for (i, &r) in roi.iter().enumerate() {
                sum_roi_per_interval[i] += r;
            }
        }
    }

    // for i in 0..TIME_ROUNDS {
    //     total_profit_per_interval[i] =
    //         sum_profit_per_interval[i] / tokens_sold_at_this_interval[i] as f32;
    // }

    for i in 0..TIME_ROUNDS {
        average_roi_per_interval[i] =
            sum_roi_per_interval[i] / tokens_sold_at_this_interval[i] as f32;
    }
    println!("----------------------------------------------");
    println!("------PROFIT PERFORMANCE BY TIME INTERVAL-----");
    println!("----------------------------------------------");

    for i in 1..TIME_ROUNDS {
        println!(
            "{} secs => profit of {}, and roi of {}",
            time_bought * i as u32,
            sum_profit_per_interval[i],
            average_roi_per_interval[i]
        );
        println!("----------------------------------------------");
    }

    Ok(())
}

pub async fn display_token_stats() -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    let mut total_profit = 0.0;
    let mut sum_roi = 0.0;
    let mut tokens_sold: u32 = 0;
    println!("----------------------------------------------");
    println!("----------------TOKEN STATS------------------");
    println!("----------------------------------------------");
    for token in tokens.values() {
        let (profits, roi) = token.display_token_portfolio().await?;

        if profits != 0.0 {
            total_profit += profits;
            sum_roi += roi;
            tokens_sold += 1;
        }
    }

    let avg_roi = if tokens_sold > 0 {
        sum_roi / tokens_sold as f64
    } else {
        0.0
    };
    let avg_profit = if tokens_sold > 0 {
        total_profit / tokens_sold as f64
    } else {
        0.0
    };

    println!("----------------------------------------------");
    println!("------PROFIT PERFORMANCE ---------------------");
    println!("----------------------------------------------");

    println!("profit of {}, and roi of {}", total_profit, avg_roi);
    println!(
        "{} tokens sold, {} profit per token",
        tokens_sold, avg_profit
    );

    println!("----------------------------------------------");
    println!("----------------------------------------------");

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
            let total_supply = token.get_total_supply(client).await?;

            if total_supply > U256::from(0) {
                token.is_tradable = true;
                info!("{} is tradable", token.name);
            } else {
                // info!("{} is not tradable", token.name);
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
                if token.is_tradable && token.state == TokenState::NotValidated {
                    token.set_state_to_(TokenState::Validating).await;

                    let token_status = token
                        .validate_with_simulated_buy_sell(TokenLiquidity::HasEnough)
                        .await?;
                    if token_status == TokenStatus::Legit {
                        info!("{} is validated!", token.name);
                        token.set_state_to_(TokenState::Validating).await;
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

pub async fn get_number_of_tokens() -> usize {
    let token_data_hash = Arc::clone(&TOKEN_HASH);
    let tokens = token_data_hash.lock().await;

    tokens.len()
}

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

    pub async fn set_to_tradable(&self) {
        let token_data_hash = Arc::clone(&TOKEN_HASH);
        let mut tokens = token_data_hash.lock().await;
        let token_address_string = self.lowercase_address();

        match tokens.get_mut(&token_address_string) {
            Some(token) => {
                token.is_tradable = true;
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
