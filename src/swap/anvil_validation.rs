use super::anvil_simlator::AnvilSimulator;
use crate::data::tokens::Erc20Token;
use ethers::types::U256;

#[derive(Debug, PartialEq, Eq)]
pub enum TokenStatus {
    Legit,
    CannotSell,
    CannotBuy,
}

impl AnvilSimulator {
    /// Takes a snapshot of the current blockchain state using anvil
    pub async fn validate_token_with_simulated_buy_sell(
        &self,
        token: &Erc20Token,
    ) -> anyhow::Result<TokenStatus> {
        // Take a snapshot before buying
        let snapshot_id = self.take_snapshot().await?;

        // Try to buy the token
        let balance_before = self.get_token_balance(token).await?;
        let buy_result = self.simulate_buying_token_for_weth(token).await;

        if let Err(err) = buy_result {
            println!("Buy transaction failed with error: {:?}", err);
            // If buying fails, revert to the snapshot so no state is changed
            self.revert_snapshot(&snapshot_id).await?;
            return Ok(TokenStatus::CannotBuy);
        }

        let balance_after_buy = self.get_token_balance(token).await?;
        if balance_after_buy <= balance_before {
            println!("No tokens received after buy, reverting...");
            // revert if something suspicious
            self.revert_snapshot(&snapshot_id).await?;
            return Ok(TokenStatus::CannotBuy);
        }

        // Now attempt to sell
        let sell_result = self.simulate_selling_token_for_weth(token).await;
        match sell_result {
            Ok(_) => {
                let balance_after_sell = self.get_token_balance(token).await?;
                if balance_after_sell != U256::from(0) {
                    println!("cannot sell {}, scam alert", token.name);
                    // If you must revert because the sale is unsuccessful, do it here
                    self.revert_snapshot(&snapshot_id).await?;
                    return Ok(TokenStatus::CannotSell);
                }

                println!("{} is legit", token.name);
                self.revert_snapshot(&snapshot_id).await?;
                Ok(TokenStatus::Legit)
            }
            Err(err) => {
                println!("Sell transaction failed: {:?}", err);
                // Revert to the snapshot taken before buying
                self.revert_snapshot(&snapshot_id).await?;
                Err(err)
            }
        }
    }
}
