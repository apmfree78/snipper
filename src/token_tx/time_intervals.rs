use crate::data::token_data::get_tokens;
use crate::data::tokens::{Erc20Token, TokenState};
use crate::utils::type_conversion::get_time_interval;
use ethers::providers::{Provider, Ws};
use ethers::utils::format_units;
use std::sync::Arc;

//****************************************************************************************
//****************************************************************************************
//****************************************************************************************
//******************** TIME BUY SELL ***************************************************
//****************************************************************************************
//****************************************************************************************
pub async fn mock_sell_eligible_tokens_at_time_intervals(
    client: &Arc<Provider<Ws>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    // println!("finding tokens to sell");
    for token in tokens.values() {
        if token.state == TokenState::Bought {
            token
                .mock_sell_at_time_intervals(client, current_time)
                .await?;
        }
    }

    // println!("done with selling...");
    Ok(())
}

impl Erc20Token {
    pub async fn mock_sell_at_time_intervals(
        &self,
        client: &Arc<Provider<Ws>>,
        current_time: u32,
    ) -> anyhow::Result<()> {
        // there are TIME_ROUND intervals separated by TOKEN_TIME_INTERVAL (env variable) mins
        let interval = get_time_interval(current_time, self.time_of_purchase)?;

        match interval {
            Some(time_index) => {
                // self.amount_sold_at_time[time_index] == U256::zero() then mock purchase already
                // complete
                if time_index > 0 && !self.is_sold_at_time[time_index] {
                    self.set_state_to_(TokenState::Selling).await;
                    let amount_sold = self.mock_sell_for_eth(client).await?;
                    let sold = format_units(amount_sold, "ether")?;
                    println!("sold {} at time index: {}", sold, time_index);

                    self.update_post_time_sale(amount_sold, time_index).await;
                }
            }
            None => {
                println!("interval returned None");
                self.set_state_to_(TokenState::Sold).await;
            }
        }

        // let token = remove_token(token.address).await.unwrap();
        // println!("token {} sold and removed!", token.name);

        Ok(())
    }
}
