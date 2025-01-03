use crate::data::token_data::{get_tokens, update_token};
use crate::data::tokens::{Erc20Token, TokenState};
use crate::utils::type_conversion::get_time_interval;
use ethers::{
    core::types::U256,
    providers::{Provider, Ws},
};
use std::sync::Arc;

pub const TIME_ROUNDS: usize = 12;
//****************************************************************************************
//****************************************************************************************
//****************************************************************************************
//******************** TIME BUY SELL ***************************************************
//****************************************************************************************
//****************************************************************************************
pub async fn sell_eligible_tokens_at_time_intervals(
    client: &Arc<Provider<Ws>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    println!("finding tokens to sell");
    for token in tokens.values() {
        if token.state == TokenState::Bought {
            print!("mock selling now...");
            token
                .mock_sell_at_time_intervals(client, current_time)
                .await?;
        }
    }

    println!("done with selling...");
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
                if time_index > 0 {
                    self.set_state_to_(TokenState::Selling).await;
                    let amount_sold = self.mock_sell_for_eth(client).await?;
                    println!("sold at time index: {}", time_index);

                    let mut current_amounts_sold = self.amount_sold_at_time.clone();

                    if current_amounts_sold[time_index] == U256::zero() {
                        current_amounts_sold[time_index] = amount_sold;

                        let updated_token = Erc20Token {
                            amount_sold_at_time: current_amounts_sold,
                            ..self.clone()
                        };
                        update_token(&updated_token).await;
                    }
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
