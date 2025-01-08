use crate::app_config::{AppMode, APP_MODE};
use crate::data::contracts::CONTRACT;
use crate::data::token_data::{get_tokens, remove_token};
use crate::data::tokens::{Erc20Token, TokenState};
use crate::swap::anvil::validation::{TokenLiquid, TokenStatus};
use crate::swap::mainnet::setup::TxWallet;
use crate::utils::tx::{
    amount_of_token_to_purchase, get_amount_out_uniswap_v2, token_tx_profit_loss, TxSlippage,
};
use ethers::types::Address;
use ethers::utils::format_units;
use ethers::{
    core::types::U256,
    providers::{Provider, Ws},
};
use log::{error, info, warn};
use std::sync::Arc;

//****************************************************************************************
//****************************************************************************************
//****************************************************************************************
//******************** BUY SELL TOKENS - NO INTERVALS ************************************
//****************************************************************************************
//****************************************************************************************
impl Erc20Token {
    pub async fn purchase(
        &self,
        tx_wallet: &Arc<TxWallet>,
        current_time: u32,
    ) -> anyhow::Result<()> {
        self.set_state_to_(TokenState::Buying).await;

        let token_balance = if APP_MODE == AppMode::Production {
            tx_wallet.buy_tokens_for_eth(self).await?
        } else {
            // simulation mode
            self.mock_buy_with_eth(&tx_wallet.client).await?
        };

        if token_balance > U256::from(0) {
            let updated_token = Erc20Token {
                is_tradable: true,
                amount_bought: token_balance,
                time_of_purchase: current_time,
                state: TokenState::Bought,
                ..self.clone()
            };

            updated_token.update_state().await;
            // info!("token updated and saved");
        } else {
            warn!("{} token purchase failed, removing", self.name);
            remove_token(self.address).await;
        }

        Ok(())
    }

    pub async fn sell(&self, tx_wallet: &Arc<TxWallet>) -> anyhow::Result<()> {
        self.set_state_to_(TokenState::Selling).await;
        let eth_revenue_from_sale = if APP_MODE == AppMode::Production {
            tx_wallet.sell_token_for_eth(self).await?
        } else {
            // simulation mode
            self.mock_sell_for_eth(&tx_wallet.client).await?
        };

        if eth_revenue_from_sale > U256::zero() {
            let updated_token = Erc20Token {
                eth_recieved_at_sale: eth_revenue_from_sale,
                state: TokenState::Sold,
                ..self.clone()
            };
            updated_token.update_state().await;
            info!("token {} sold!", self.name);
        } else {
            let updated_token = Erc20Token {
                eth_recieved_at_sale: U256::zero(),
                state: TokenState::Sold,
                ..self.clone()
            };
            updated_token.update_state().await;
            warn!("failed to sell token, rug pull => {}", self.name);
        }

        Ok(())
    }

    pub async fn mock_buy_with_eth(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<U256> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        println!("........................................................");

        let amount_in = amount_of_token_to_purchase()?;

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

    pub async fn mock_sell_for_eth(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<U256> {
        // check that token has liquidity
        let amount_to_sell = self.amount_bought;

        let has_enough_liquidity = self
            .has_enough_liquidity_for_trade(amount_to_sell, client)
            .await?;

        if !has_enough_liquidity {
            println!(".............RUG PULL.....................");
            return Ok(U256::zero());
        }

        // now validate token is not rugged
        println!("re-validating token {}", self.name);
        let token_status = self
            .validate_with_simulated_buy_sell(TokenLiquid::HasEnough)
            .await?;

        if token_status != TokenStatus::Legit {
            println!(".............RUG PULL.....................");
            println!("{} failed re-validation", self.name);
            return Ok(U256::zero());
        }
        println!("{} successfully re-validated", self.name);

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
        let profit = token_tx_profit_loss(amount_out)?;
        println!(
            "sold {} for {} eth with profit {}",
            self.name, amount_out_min_readable, profit
        );
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

pub async fn buy_eligible_tokens(tx_wallet: &Arc<TxWallet>, timestamp: u32) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    // println!("finding tokens to buy");
    for token in tokens.values() {
        if token.is_tradable && token.state == TokenState::Locked {
            let spawn_token = token.clone();
            let spawn_tx_wallet = Arc::clone(tx_wallet);
            tokio::spawn(async move {
                if let Err(error) = spawn_token.purchase(&spawn_tx_wallet, timestamp).await {
                    error!("could not purchase token => {}", error);
                }
            });
        }
    }
    // println!("done with purchasing...");
    Ok(())
}

pub async fn sell_eligible_tokens(
    tx_wallet: &Arc<TxWallet>,
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
            let spawn_token = token.clone();
            let spawn_tx_wallet = Arc::clone(tx_wallet);
            tokio::spawn(async move {
                if let Err(error) = spawn_token.sell(&spawn_tx_wallet).await {
                    error!("could not sell token => {}", error);
                }
            });
        }
    }

    // println!("done with selling...");
    Ok(())
}
