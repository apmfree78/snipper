use crate::data::contracts::CONTRACT;
use crate::data::token_data::{get_tokens, set_token_to_, update_token};
use crate::data::tokens::{Erc20Token, TokenState};
use crate::utils::tx::{get_amount_out_uniswap_v2, TxSlippage};
use ethers::types::Address;
use ethers::utils::format_units;
use ethers::{
    core::types::U256,
    providers::{Provider, Ws},
};
use log::info;
use std::sync::Arc;

//****************************************************************************************
//****************************************************************************************
//****************************************************************************************
//******************** BUY SELL TOKENS - NO INTERVALS ************************************
//****************************************************************************************
//****************************************************************************************
impl Erc20Token {
    pub async fn mock_purchase(
        &self,
        client: &Arc<Provider<Ws>>,
        current_time: u32,
    ) -> anyhow::Result<()> {
        let token_balance = self.mock_buy_with_weth(client).await?;

        if token_balance > U256::from(0) {
            let updated_token = Erc20Token {
                is_tradable: true,
                amount_bought: token_balance,
                time_of_purchase: current_time,
                state: TokenState::Bought,
                ..self.clone()
            };

            update_token(&updated_token).await;
            info!("token updated and saved");
        }

        Ok(())
    }

    pub async fn mock_sell(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<()> {
        let eth_revenue_from_sale = self.mock_sell_for_weth(client).await?;

        if eth_revenue_from_sale > U256::zero() {
            let updated_token = Erc20Token {
                eth_recieved_at_sale: eth_revenue_from_sale,
                ..self.clone()
            };
            update_token(&updated_token).await;
        }

        set_token_to_(TokenState::Sold, self).await;
        info!("token {} sold!", self.name);

        Ok(())
    }

    pub async fn mock_buy_with_weth(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<U256> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        println!("........................................................");
        let amount_to_buy =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        println!("buying {} WETH of {}", amount_to_buy, self.name);
        let amount_in = ethers::utils::parse_ether(amount_to_buy)?;

        // calculate amount amount out and gas used
        println!("........................................................");
        let amount_out = get_amount_out_uniswap_v2(
            weth_address,
            self.address,
            amount_in,
            TxSlippage::None,
            client,
        )
        .await?;

        let amount_out_readable = format_units(amount_out, u32::from(self.decimals))?;
        println!("bought {} of {}", amount_out_readable, self.name);
        println!("........................................................");
        Ok(amount_out)
    }

    pub async fn mock_sell_for_weth(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<U256> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        println!("........................................................");
        let amount_to_sell = self.amount_bought;

        //approve swap router to trade toke
        println!("........................................................");
        let amount_out = get_amount_out_uniswap_v2(
            self.address,
            weth_address,
            amount_to_sell,
            TxSlippage::None,
            client,
        )
        .await?;

        let amount_out_min_readable = format_units(amount_out, 18u32)?;
        println!("sold {} for {} eth", self.name, amount_out_min_readable);
        println!("........................................................");

        Ok(amount_out)
    }
}

//****************************************************************************************
//****************************************************************************************
//****************************************************************************************
//******************** LOOP THROUGH ALL TOKENS TO BUY SELL *******************************
//****************************************************************************************
//****************************************************************************************

pub async fn mock_buy_eligible_tokens(
    client: &Arc<Provider<Ws>>,
    timestamp: u32,
) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    println!("finding tokens to buy");
    for token in tokens.values() {
        if token.is_tradable && token.state == TokenState::Validated {
            token.mock_purchase(client, timestamp).await?;
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

        if token.state == TokenState::Bought && current_time >= sell_time {
            token.mock_sell(client).await?;
        }
    }

    println!("done with selling...");
    Ok(())
}

