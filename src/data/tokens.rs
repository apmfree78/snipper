use super::token_data::{get_and_save_erc20_by_token_address, get_tokens, update_token};
use crate::data::token_data::remove_token;
use crate::events::PairCreatedEvent;
use crate::swap::anvil_simlator::AnvilSimulator;
use crate::swap::token_price::get_token_weth_total_supply;
use ethers::{
    abi::Address,
    core::types::U256,
    providers::{Provider, Ws},
};
use log::info;
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub struct Erc20Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub address: Address,
    pub pair_address: Address,
    pub is_tradable: bool,
    pub is_token_0: bool,
    pub done_buying: bool,
    pub amount_bought: U256,
    pub time_of_purchase: u32,
}

pub async fn add_validate_buy_new_token(
    pair_created_event: &PairCreatedEvent,
    client: &Arc<Provider<Ws>>,
    anvil: &Arc<AnvilSimulator>,
    current_time: u32,
) -> anyhow::Result<()> {
    // TODO - VALIDATE TOKEN HERE - IF SCAM exit out

    // SAVE TOKEN TO GLOBAL STATE
    if let Some(token) =
        get_and_save_erc20_by_token_address(&pair_created_event, client, anvil).await?
    {
        // check liqudity
        let total_supply = get_token_weth_total_supply(&token, client).await?;

        if total_supply > U256::from(0) {
            info!(
                "{} has immediate liquidity of {} and ready for trading",
                token.name, total_supply
            );
            purchase_token_on_anvil(&token, anvil, current_time).await?;
        } else {
            info!("{} has no liquidity, cannot purchase yet!", token.name);
        }
    }

    Ok(())
}

pub async fn buy_eligible_tokens_on_anvil(
    anvil: &Arc<AnvilSimulator>,
    timestamp: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    println!("finding tokens to buy");
    for token in tokens.values() {
        if !token.done_buying && token.is_tradable {
            purchase_token_on_anvil(token, anvil, timestamp).await?;
        }
    }
    println!("done with purchasing...");
    Ok(())
}

pub async fn sell_eligible_tokens_on_anvil(
    anvil: &Arc<AnvilSimulator>,
    current_time: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;
    let time_to_sell =
        std::env::var("SELL_TOKEN_AFTER").expect("SELL_TOKEN_AFTER not found in .env");
    let time_to_sell: u32 = time_to_sell.parse()?;

    println!("finding tokens to sell");
    for token in tokens.values() {
        let sell_time = time_to_sell + token.time_of_purchase;

        if token.done_buying && current_time >= sell_time {
            sell_token_on_anvil(token, anvil).await?;
        }
    }

    println!("done with selling...");
    Ok(())
}

pub async fn purchase_token_on_anvil(
    token: &Erc20Token,
    anvil: &Arc<AnvilSimulator>,
    current_time: u32,
) -> anyhow::Result<()> {
    let token_balance = anvil.simulate_buying_token_for_weth(&token).await?;

    if token_balance > U256::from(0) {
        let updated_token = Erc20Token {
            is_tradable: true,
            amount_bought: token_balance,
            time_of_purchase: current_time,
            done_buying: true,
            ..token.clone()
        };

        update_token(&updated_token).await;
        info!("token updated and saved");
    }

    Ok(())
}

pub async fn sell_token_on_anvil(
    token: &Erc20Token,
    anvil: &Arc<AnvilSimulator>,
) -> anyhow::Result<()> {
    let token_balance = anvil.simulate_selling_token_for_weth(&token).await?;

    if token_balance == U256::from(0) {
        let token = remove_token(token.address).await.unwrap();
        info!("token {} sold and removed!", token.name);
    }

    Ok(())
}
