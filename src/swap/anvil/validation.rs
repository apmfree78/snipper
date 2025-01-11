use super::simlator::AnvilSimulator;
use crate::data::{contracts::CONTRACT, tokens::Erc20Token};
use crate::swap::tx_trait::Txs;
use ethers::providers::Middleware;
use ethers::types::{Transaction, U256};
use log::{error, info, warn};

#[derive(Debug, PartialEq, Eq)]
pub enum TokenStatus {
    Legit,
    CannotSell,
    CannotBuy,
}

pub enum TokenLiquid {
    NeedToAdd(Transaction),
    HasEnough,
}

impl Erc20Token {
    /// Takes a snapshot of the current blockchain state using anvil
    pub async fn validate_with_simulated_buy_sell(
        &self,
        liquidity_status: TokenLiquid,
    ) -> anyhow::Result<TokenStatus> {
        // launch new anvil node for validation
        let ws_url = CONTRACT.get_address().alchemy_url.clone();
        let anvil = AnvilSimulator::new(&ws_url).await?;

        println!("validating token...");

        match liquidity_status {
            TokenLiquid::NeedToAdd(add_liquidity_tx) => {
                // simulate adding liquidity
                println!("simulate adding liquidity before buying");
                anvil.add_liquidity_eth(&add_liquidity_tx).await?;
            }
            TokenLiquid::HasEnough => {}
        }

        // Try to buy the token
        // let balance_before = anvil.get_token_balance(token).await?;
        println!("simulate buying token for validation");
        let buy_result = anvil.simulate_buying_token_for_weth(self).await;

        if let Err(err) = buy_result {
            error!("Buy transaction failed with error: {:?}", err);
            // If buying fails, revert to the snapshot so no state is changed
            return Ok(TokenStatus::CannotBuy);
        }

        let token_balance = buy_result?;

        println!("check token balance after purchase");
        if token_balance == U256::from(0) {
            println!("No tokens received after buy, reverting...");
            // revert if something suspicious
            return Ok(TokenStatus::CannotBuy);
        }

        // Increase time by 300 seconds (5 minutes)
        println!("simulating time elapse of 5 mins");
        anvil
            .signed_client
            .provider()
            .request::<_, u64>("evm_increaseTime", [300u64])
            .await?;

        // elapse time with blocks mine
        println!("simulating creating blocks");
        for _ in 0..100 {
            let _ = anvil
                .signed_client
                .provider()
                .request::<_, String>("evm_mine", None::<()>)
                .await?;
        }

        println!("check token balance after five minutes");
        // check token balance did not drop after time elapse
        let balance_after_buy = anvil
            .get_wallet_token_balance_by_address(self.address)
            .await?;
        if balance_after_buy < token_balance {
            println!("Token are dropping or going to zero after 5 mins...");
            // revert if something suspicious
            return Ok(TokenStatus::CannotBuy);
        }

        // simulate transfer between wallets
        anvil
            .do_dummy_transfer(self.address, balance_after_buy)
            .await?;

        println!("check token balance after transfers");
        // check token balance did not drop after time elapse
        let balance_after_transfer = anvil
            .get_wallet_token_balance_by_address(self.address)
            .await?;
        if balance_after_transfer < token_balance {
            println!("Token are dropping or going to zero after transfer...");
            // revert if something suspicious
            return Ok(TokenStatus::CannotBuy);
        }

        // Now attempt to sell
        // info!("simulate selling token for validation");
        let sell_result = anvil.simulate_selling_token_for_weth(self).await;
        match sell_result {
            Ok(_) => {
                println!("check token balance after sale (should be zero)");
                let balance_after_sell = anvil
                    .get_wallet_token_balance_by_address(self.address)
                    .await?;
                if balance_after_sell != U256::from(0) {
                    println!("cannot sell {}, scam alert", self.name);
                    // If you must revert because the sale is unsuccessful, do it here
                    return Ok(TokenStatus::CannotSell);
                }

                println!("{} is legit", self.name);
                Ok(TokenStatus::Legit)
            }
            Err(err) => {
                println!("Sell transaction failed: {:?}", err);
                Err(err)
            }
        }
    }
}
