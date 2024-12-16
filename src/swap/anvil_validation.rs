use super::anvil_simlator::AnvilSimulator;
use crate::data::{contracts::CONTRACT, tokens::Erc20Token};
use ethers::types::U256;
use log::info;

#[derive(Debug, PartialEq, Eq)]
pub enum TokenStatus {
    Legit,
    CannotSell,
    CannotBuy,
}

/// Takes a snapshot of the current blockchain state using anvil
pub async fn validate_token_with_simulated_buy_sell(
    token: &Erc20Token,
) -> anyhow::Result<TokenStatus> {
    // launch new anvil node for validation
    let ws_url = CONTRACT.get_address().ws_url.clone();
    let anvil = AnvilSimulator::new(&ws_url).await?;

    // Try to buy the token
    // let balance_before = anvil.get_token_balance(token).await?;
    info!("simulate buying token for validation");
    let buy_result = anvil.simulate_buying_token_for_weth(token).await;

    if let Err(err) = buy_result {
        println!("Buy transaction failed with error: {:?}", err);
        // If buying fails, revert to the snapshot so no state is changed
        return Ok(TokenStatus::CannotBuy);
    }

    let balance_after_buy = anvil.get_token_balance(token).await?;
    if balance_after_buy == U256::from(0) {
        println!("No tokens received after buy, reverting...");
        // revert if something suspicious
        return Ok(TokenStatus::CannotBuy);
    }

    // Now attempt to sell
    info!("simulate selling token for validation");
    let sell_result = anvil.simulate_selling_token_for_weth(token).await;
    match sell_result {
        Ok(_) => {
            let balance_after_sell = anvil.get_token_balance(token).await?;
            if balance_after_sell != U256::from(0) {
                println!("cannot sell {}, scam alert", token.name);
                // If you must revert because the sale is unsuccessful, do it here
                return Ok(TokenStatus::CannotSell);
            }

            println!("{} is legit", token.name);
            Ok(TokenStatus::Legit)
        }
        Err(err) => {
            println!("Sell transaction failed: {:?}", err);
            // Revert to the snapshot taken before buying
            Err(err)
        }
    }
}
