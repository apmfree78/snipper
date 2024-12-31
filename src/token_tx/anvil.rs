use crate::data::token_data::remove_token;
use crate::data::token_data::{get_tokens, update_token};
use crate::data::tokens::{Erc20Token, TokenState};
use crate::swap::anvil::simlator::AnvilSimulator;
use ethers::core::types::U256;
use futures::lock::Mutex;
use log::info;
use std::sync::Arc;
// ---------------------------------------------------------------
// ---------------------------------------------------------------
// ---------------------------------------------------------------
// ---------------------------------------------------------------
// ---------------------------------------------------------------
// DECPRECIATED ANVIL METHODS -------------
// ---------------------------------------------------------------
// ---------------------------------------------------------------
// ---------------------------------------------------------------
// ---------------------------------------------------------------
// ---------------------------------------------------------------
pub async fn buy_eligible_tokens_on_anvil(
    anvil: &Arc<Mutex<AnvilSimulator>>,
    timestamp: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    println!("finding tokens to buy");
    for token in tokens.values() {
        if token.is_tradable && token.state == TokenState::Validated {
            purchase_token_on_anvil(token, anvil, timestamp).await?;
        }
    }
    println!("done with purchasing...");
    Ok(())
}

pub async fn sell_eligible_tokens_on_anvil(
    anvil: &Arc<Mutex<AnvilSimulator>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;
    let time_to_sell =
        std::env::var("SELL_TOKEN_AFTER").expect("SELL_TOKEN_AFTER not found in .env");
    let time_to_sell: u32 = time_to_sell.parse()?;

    println!("finding tokens to sell");
    for token in tokens.values() {
        let sell_time = time_to_sell + token.time_of_purchase;

        if token.state == TokenState::Bought && current_time >= sell_time {
            sell_token_on_anvil(token, anvil).await?;
        }
    }

    println!("done with selling...");
    Ok(())
}

pub async fn purchase_token_on_anvil(
    token: &Erc20Token,
    anvil: &Arc<Mutex<AnvilSimulator>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let anvil_lock = anvil.lock().await;
    let token_balance = anvil_lock.simulate_buying_token_for_weth(&token).await?;

    if token_balance > U256::from(0) {
        let updated_token = Erc20Token {
            is_tradable: true,
            amount_bought: token_balance,
            time_of_purchase: current_time,
            state: TokenState::Bought,
            ..token.clone()
        };

        update_token(&updated_token).await;
        info!("token updated and saved");
    }

    Ok(())
}

pub async fn sell_token_on_anvil(
    token: &Erc20Token,
    anvil: &Arc<Mutex<AnvilSimulator>>,
) -> anyhow::Result<()> {
    let anvil_lock = anvil.lock().await;

    let token_balance = anvil_lock.simulate_selling_token_for_weth(&token).await?;

    if token_balance == U256::from(0) {
        let token = remove_token(token.address).await.unwrap();
        info!("token {} sold and removed!", token.name);
    }

    Ok(())
}
