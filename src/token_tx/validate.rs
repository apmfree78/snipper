use crate::data::token_data::get_and_save_erc20_by_token_address;
use crate::data::token_data::remove_token;
use crate::data::token_data::set_token_to_;
use crate::data::token_data::set_token_to_tradable;
use crate::data::tokens::Erc20Token;
use crate::data::tokens::TokenState;
use crate::events::PairCreatedEvent;
use crate::swap::anvil::validation::TokenLiquidity;
use crate::swap::anvil::validation::TokenStatus;
use crate::swap::mainnet::setup::TxWallet;
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
    tx_wallet: &Arc<TxWallet>,
    current_time: u32,
) -> anyhow::Result<()> {
    // SAVE TOKEN TO GLOBAL STATE
    if let Some(token) =
        get_and_save_erc20_by_token_address(&pair_created_event, &tx_wallet.client).await?
    {
        // check liqudity
        let total_supply = token.get_total_supply(&tx_wallet.client).await?;

        if total_supply > U256::from(0) {
            set_token_to_tradable(&token).await;
            info!(
                "{} has immediate liquidity of {} and ready for trading",
                token.name, total_supply
            );

            set_token_to_(TokenState::Validating, &token).await;
            let token_status = token
                .validate_with_simulated_buy_sell(TokenLiquidity::HasEnough)
                .await?;

            if token_status == TokenStatus::Legit {
                set_token_to_(TokenState::Validated, &token).await;
                token.mock_purchase(&tx_wallet.client, current_time).await?;
            } else {
                // cannot buy or sell token remove it
                let removed_token = remove_token(token.address).await.unwrap();
                warn!("scam token {} removed", removed_token.name);
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
    tx_wallet: &Arc<TxWallet>,
    current_time: u32,
) -> anyhow::Result<()> {
    if token.state != TokenState::NotValidated {
        return Ok(());
    }

    set_token_to_(TokenState::Validating, &token).await;
    let token_status = token
        .validate_with_simulated_buy_sell(TokenLiquidity::NeedToAdd(add_liquidity_tx.clone()))
        .await?;

    if token_status == TokenStatus::Legit {
        info!("{} token validated from mempool!", token.name);
        set_token_to_(TokenState::Validated, token).await;

        // check if token is tradable
        let total_supply = token.get_total_supply(&tx_wallet.client).await?;
        if total_supply > U256::from(0) {
            set_token_to_tradable(&token).await;

            // go ahead and purchase
            token.mock_purchase(&tx_wallet.client, current_time).await?;
        }
    } else {
        let scam_token = remove_token(token.address).await;
        let scam_token = scam_token.unwrap();
        warn!("removed (mempool) {}", scam_token.symbol);
    }
    Ok(())
}
