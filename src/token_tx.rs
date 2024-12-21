use crate::data::token_data::{get_and_save_erc20_by_token_address, get_tokens, update_token};
use crate::data::token_data::{remove_token, set_token_to_validated};
use crate::data::tokens::Erc20Token;
use crate::events::PairCreatedEvent;
use crate::swap::anvil_simlator::AnvilSimulator;
use crate::swap::anvil_validation::{validate_token_with_simulated_buy_sell, TokenStatus};
use crate::swap::token_price::get_token_weth_total_supply;
use ethers::{
    core::types::U256,
    providers::{Provider, Ws},
};
use futures::lock::Mutex;
use log::info;
use std::sync::Arc;

pub async fn add_validate_buy_new_token(
    pair_created_event: &PairCreatedEvent,
    client: &Arc<Provider<Ws>>,
    current_time: u32,
) -> anyhow::Result<()> {
    // SAVE TOKEN TO GLOBAL STATE
    if let Some(token) = get_and_save_erc20_by_token_address(&pair_created_event, client).await? {
        // check liqudity
        let total_supply = get_token_weth_total_supply(&token, client).await?;

        if total_supply > U256::from(0) {
            info!(
                "{} has immediate liquidity of {} and ready for trading",
                token.name, total_supply
            );

            let token_status = validate_token_with_simulated_buy_sell(&token).await?;

            if token_status == TokenStatus::Legit {
                set_token_to_validated(&token).await;
                mock_purchase_token(&token, client, current_time).await?;
                // purchase_token_on_anvil(&token, anvil, current_time).await?;
            }
        } else {
            info!("{} has no liquidity, cannot purchase yet!", token.name);
        }
    }

    Ok(())
}

pub async fn mock_purchase_token(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let token_balance = token.mock_buy_with_weth(client).await?;

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

pub async fn mock_purchase_tokens_for_volume_interval(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let token_balances = token.mock_multiple_buys_with_weth(client).await?;

    let updated_token = Erc20Token {
        is_tradable: true,
        amounts_bought: token_balances,
        time_of_purchase: current_time,
        done_buying: true,
        ..token.clone()
    };

    update_token(&updated_token).await;
    println!("token updated and saved");

    Ok(())
}

pub async fn mock_sell_token(token: &Erc20Token, client: &Arc<Provider<Ws>>) -> anyhow::Result<()> {
    let eth_revenue_from_sale = token.mock_sell_for_weth(client).await?;

    if eth_revenue_from_sale > U256::zero() {
        let updated_token = Erc20Token {
            eth_recieved_at_sale: eth_revenue_from_sale,
            ..token.clone()
        };
        update_token(&updated_token).await;
    }

    let token = remove_token(token.address).await.unwrap();
    info!("token {} sold and removed!", token.name);

    Ok(())
}

pub async fn mock_sell_tokens_volume_interval(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<()> {
    let amounts_sold = token.mock_multiple_sells_for_weth(client).await?;

    let updated_token = Erc20Token {
        amounts_sold,
        ..token.clone()
    };
    update_token(&updated_token).await;

    // let token = remove_token(token.address).await.unwrap();
    // println!("token {} sold and removed!", token.name);

    Ok(())
}

pub async fn mock_buy_eligible_tokens(
    client: &Arc<Provider<Ws>>,
    timestamp: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    println!("finding tokens to buy");
    for token in tokens.values() {
        if !token.done_buying && token.is_tradable && token.is_validated {
            mock_purchase_tokens_for_volume_interval(&token, client, timestamp).await?;
        }
    }
    println!("done with purchasing...");
    Ok(())
}

pub async fn mock_sell_eligible_tokens(
    client: &Arc<Provider<Ws>>,
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
            mock_sell_tokens_volume_interval(&token, client).await?;
        }
    }

    println!("done with selling...");
    Ok(())
}

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
        if !token.done_buying && token.is_tradable && token.is_validated {
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

        if token.done_buying && current_time >= sell_time {
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
