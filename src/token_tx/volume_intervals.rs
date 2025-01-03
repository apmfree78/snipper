use crate::data::contracts::CONTRACT;
use crate::data::token_data::get_tokens;
use crate::data::tokens::{Erc20Token, TokenState};
use crate::utils::tx::{get_amount_out_uniswap_v2, TxSlippage};
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use ethers::utils::format_units;
use std::sync::Arc;

pub const VOLUME_ROUNDS: usize = 5;

//****************************************************************************************
//****************************************************************************************
//****************************************************************************************
//******************** VOLUME BUY SELL ***************************************************
//****************************************************************************************
//****************************************************************************************
pub async fn mock_buy_eligible_tokens_at_volume_interval(
    client: &Arc<Provider<Ws>>,
    timestamp: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    // println!("finding tokens to buy");
    for token in tokens.values() {
        if token.is_tradable && token.state == TokenState::Validated {
            token
                .mock_purchase_at_volume_intervals(client, timestamp)
                .await?;
        }
    }
    // println!("done with purchasing...");
    Ok(())
}

pub async fn mock_sell_eligible_tokens_at_volume_interval(
    client: &Arc<Provider<Ws>>,
    current_time: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;
    let time_to_sell =
        std::env::var("SELL_TOKEN_AFTER").expect("SELL_TOKEN_AFTER not found in .env");
    let time_to_sell: u32 = time_to_sell.parse()?;

    // println!("finding tokens to sell");
    for token in tokens.values() {
        let sell_time = time_to_sell + token.time_of_purchase;

        if token.state == TokenState::Bought && current_time >= sell_time {
            token.mock_sell_at_volume_intervals(client).await?;
        }
    }

    // println!("done with selling...");
    Ok(())
}

impl Erc20Token {
    pub async fn mock_purchase_at_volume_intervals(
        &self,
        client: &Arc<Provider<Ws>>,
        current_time: u32,
    ) -> anyhow::Result<()> {
        let token_balances = self.mock_multiple_buys_with_weth(client).await?;

        let updated_token = Erc20Token {
            is_tradable: true,
            amounts_bought: token_balances,
            time_of_purchase: current_time,
            state: TokenState::Bought,
            ..self.clone()
        };

        updated_token.update_state().await;
        println!("token updated and saved");

        Ok(())
    }

    pub async fn mock_sell_at_volume_intervals(
        &self,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<()> {
        let amounts_sold = self.mock_multiple_sells_for_weth(client).await?;

        let updated_token = Erc20Token {
            amounts_sold,
            ..self.clone()
        };
        updated_token.update_state().await;

        // let token = remove_token(token.address).await.unwrap();
        // println!("token {} sold and removed!", token.name);

        Ok(())
    }

    pub async fn mock_multiple_sells_for_weth(
        &self,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<[U256; VOLUME_ROUNDS]> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let mut amounts_out = [U256::zero(); VOLUME_ROUNDS];

        println!("........................................................");
        let amounts_to_sell = self.amounts_bought;

        for i in 0..VOLUME_ROUNDS {
            //approve swap router to trade toke
            println!("........................................................");
            let amount_out = get_amount_out_uniswap_v2(
                self.address,
                weth_address,
                amounts_to_sell[i],
                TxSlippage::None,
                client,
            )
            .await?;

            amounts_out[i] = amount_out;

            let amount_in_readable = format_units(amounts_to_sell[i], u32::from(self.decimals))?;
            let amount_out_min_readable = format_units(amount_out, 18u32)?;
            println!(
                "sold {} of {} for {} eth",
                amount_in_readable, self.name, amount_out_min_readable
            );
            println!("........................................................");
        }

        Ok(amounts_out)
    }

    pub async fn mock_multiple_buys_with_weth(
        &self,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<[U256; VOLUME_ROUNDS]> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let mut amounts_out = [U256::zero(); VOLUME_ROUNDS];

        println!("........................................................");
        let amount_to_buy =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let base_amount_in = ethers::utils::parse_ether(amount_to_buy)?;

        for i in 0..VOLUME_ROUNDS {
            println!("........................................................");
            let amount_in = U256::from(i + 1) * base_amount_in;
            let amount_out = get_amount_out_uniswap_v2(
                weth_address,
                self.address,
                amount_in,
                TxSlippage::None,
                client,
            )
            .await?;

            amounts_out[i] = amount_out;

            let amount_out_readable = format_units(amount_out, u32::from(self.decimals))?;
            let amount_in_readable = format_units(amount_in, 18u32)?;
            println!(
                "bought {} of {} with {} eth",
                amount_out_readable, self.name, amount_in_readable
            );
            println!("........................................................");
        }
        Ok(amounts_out)
    }
}
