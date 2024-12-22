use crate::data::token_data::get_and_save_erc20_by_token_address;
use crate::data::token_data::set_token_to_validated;
use crate::events::PairCreatedEvent;
use crate::swap::anvil_validation::{validate_token_with_simulated_buy_sell, TokenStatus};
use crate::swap::token_price::get_token_weth_total_supply;
use ethers::{
    core::types::U256,
    providers::{Provider, Ws},
};
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
                token.mock_purchase(client, current_time).await?;
            }
        } else {
            info!("{} has no liquidity, cannot purchase yet!", token.name);
        }
    }

    Ok(())
}
