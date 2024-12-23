use crate::data::token_data::get_and_save_erc20_by_token_address;
use crate::data::token_data::remove_token;
use crate::data::token_data::set_token_to_tradable;
use crate::data::token_data::set_token_to_validated;
use crate::data::tokens::Erc20Token;
use crate::events::PairCreatedEvent;
use crate::swap::anvil_validation::TokenLiquidity;
use crate::swap::anvil_validation::{validate_token_with_simulated_buy_sell, TokenStatus};
use crate::swap::token_price::get_token_weth_total_supply;
use ethers::types::Transaction;
use ethers::{
    core::types::U256,
    providers::{Provider, Ws},
};
use log::info;
use log::warn;
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

            let token_status =
                validate_token_with_simulated_buy_sell(&token, TokenLiquidity::HasEnough).await?;

            if token_status == TokenStatus::Legit {
                set_token_to_validated(&token).await;
                token.mock_purchase(client, current_time).await?;
            }
        } else {
            info!("{} has no liquidity, cannot purchase yet!", token.name);
        }
    }

    Ok(())
}

pub async fn validate_token_from_mempool_and_buy(
    token: &Erc20Token,
    add_liquidity_tx: &Transaction,
    client: &Arc<Provider<Ws>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let token_status = validate_token_with_simulated_buy_sell(
        token,
        TokenLiquidity::NeedToAdd(add_liquidity_tx.clone()),
    )
    .await?;

    if token_status == TokenStatus::Legit {
        info!("{} token validated from mempool!", token.name);
        set_token_to_validated(token).await;

        // check if token is tradable
        let total_supply = get_token_weth_total_supply(&token, client).await?;
        if total_supply > U256::from(0) {
            set_token_to_tradable(&token).await;

            // go ahead and purchase
            token.mock_purchase(client, current_time).await?;
        }
    } else {
        let scam_token = remove_token(token.address).await;
        let scam_token = scam_token.unwrap();
        warn!("removed (mempool) {}", scam_token.symbol);
    }
    Ok(())
}
