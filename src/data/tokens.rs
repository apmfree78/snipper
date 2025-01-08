use derive_more::Display;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use ethers::utils::format_units;
use std::sync::Arc;

use crate::abi::uniswap_pair::UNISWAP_PAIR;
use crate::app_config::{
    HIGH_LIQUIDITY_THRESHOLD, LIQUIDITY_PERCENTAGE_LOCKED, LOW_LIQUIDITY_THRESHOLD,
    MEDIUM_LIQUIDITY_THRESHOLD, MIN_LIQUIDITY, MIN_RESERVE_ETH_FACTOR, MIN_TRADE_FACTOR,
    TIME_ROUNDS, VERY_LOW_LIQUIDITY_THRESHOLD,
};
use crate::data::token_data::remove_token;
use crate::token_tx::volume_intervals::VOLUME_ROUNDS;
use crate::utils::tx::amount_of_token_to_purchase;
use crate::utils::type_conversion::address_to_string;
use crate::verify::check_token_lock::is_liquidity_locked;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum TokenState {
    #[default]
    NotValidated,
    Validating,
    Validated,
    Locked,
    Buying,
    Bought,
    Selling,
    Sold,
}

#[derive(Clone, Default, Display, Debug, PartialEq, Eq)]
pub enum TokenLiquidity {
    #[display(fmt = "Zero")]
    #[default]
    Zero,
    #[display(fmt = "Micro")]
    Micro(u128), // up to 1 ETH
    #[display(fmt = "Very Low")]
    VeryLow(u128), // 1 to 10 ETH
    #[display(fmt = "Low")]
    Low(u128), // 10 to 30 ETH
    #[display(fmt = "Medium")]
    Medium(u128), // 30 to 50 ETH
    #[display(fmt = "High")]
    High(u128), // over 50 ETH
}

#[derive(Clone, Default, Debug)]
pub struct Erc20Token {
    // basic token data
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub address: Address,

    pub pair_address: Address, // uniswap pair address
    pub is_token_0: bool,

    // token state
    pub is_tradable: bool,
    pub state: TokenState,
    pub liquidity: TokenLiquidity,
    pub amount_bought: U256,
    pub eth_recieved_at_sale: U256,
    pub time_of_purchase: u32,

    // total gas cost for buy + sell of token
    pub tx_gas_cost: U256,

    // for mock buying/selling different amounts of token
    pub amounts_bought: [U256; VOLUME_ROUNDS],
    pub amounts_sold: [U256; VOLUME_ROUNDS],

    // for mock buying/selling different times
    pub amount_sold_at_time: [U256; TIME_ROUNDS],
    pub is_sold_at_time: [bool; TIME_ROUNDS],
}

pub fn extract_liquidity_amount(liquidity: &TokenLiquidity) -> Option<u128> {
    match liquidity {
        TokenLiquidity::Zero => None,
        TokenLiquidity::Micro(value) => Some(*value),
        TokenLiquidity::VeryLow(value) => Some(*value),
        TokenLiquidity::Low(value) => Some(*value),
        TokenLiquidity::Medium(value) => Some(*value),
        TokenLiquidity::High(value) => Some(*value),
    }
}

impl Erc20Token {
    pub async fn validate_liquidity_is_locked(
        &self,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<bool> {
        match is_liquidity_locked(self, LIQUIDITY_PERCENTAGE_LOCKED, client).await? {
            Some(is_locked) => {
                if is_locked {
                    println!("{} has locked liquidity!", self.name);
                    self.set_state_to_(TokenState::Locked).await;
                } else {
                    println!("{} does not have locked liquidity... removing", self.name);
                    remove_token(self.address).await;
                }
                return Ok(is_locked);
            }
            None => {
                println!(
                    "{} waiting for graphql to provide liquidity data",
                    self.name
                );
                return Ok(false);
            }
        }
    }

    pub async fn get_total_supply(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<U256> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        // info!("getting total liquidity");
        let supply = pool.total_supply().call().await?;

        Ok(supply)
    }

    pub async fn has_enough_liquidity(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<bool> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        let (reserve0, reserve1, _) = pool.get_reserves().call().await?;

        let eth_supply = if self.is_token_0 { reserve1 } else { reserve0 };

        // let eth = eth_supply as f64 / 1e18_f64;

        Ok(eth_supply >= MIN_LIQUIDITY)
    }

    pub async fn get_liquidity(
        &self,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<TokenLiquidity> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        let (reserve0, reserve1, _) = pool.get_reserves().call().await?;

        let eth_supply = if self.is_token_0 { reserve1 } else { reserve0 };

        if eth_supply >= HIGH_LIQUIDITY_THRESHOLD {
            Ok(TokenLiquidity::High(eth_supply))
        } else if eth_supply >= MEDIUM_LIQUIDITY_THRESHOLD {
            Ok(TokenLiquidity::Medium(eth_supply))
        } else if eth_supply >= LOW_LIQUIDITY_THRESHOLD {
            Ok(TokenLiquidity::Low(eth_supply))
        } else if eth_supply >= VERY_LOW_LIQUIDITY_THRESHOLD {
            Ok(TokenLiquidity::VeryLow(eth_supply))
        } else if eth_supply != 0 {
            Ok(TokenLiquidity::Micro(eth_supply))
        } else {
            Ok(TokenLiquidity::Zero)
        }
    }

    pub async fn has_enough_liquidity_for_trade(
        &self,
        tokens_to_sell: U256,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<bool> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        let eth_amount_used_for_purchase = amount_of_token_to_purchase()?;

        let (reserve0, reserve1, _) = pool.get_reserves().call().await?;

        let (eth_supply, token_supply) = if self.is_token_0 {
            (reserve1, reserve0)
        } else {
            (reserve0, reserve1)
        };

        let enough_liquidity = tokens_to_sell * MIN_TRADE_FACTOR < U256::from(token_supply)
            && eth_amount_used_for_purchase * MIN_RESERVE_ETH_FACTOR < U256::from(eth_supply);

        Ok(enough_liquidity)
    }

    pub fn profit(&self) -> anyhow::Result<f64> {
        if self.state != TokenState::Sold {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let total_cost = eth_basis + self.tx_gas_cost;
        let profit = if self.eth_recieved_at_sale >= total_cost {
            let abs_profit = self.eth_recieved_at_sale - total_cost;
            abs_profit.as_u128() as i128
        } else {
            let abs_profit = total_cost - self.eth_recieved_at_sale;
            -(abs_profit.as_u128() as i128)
        };

        let profit = profit as f64 / 1e18_f64;

        Ok(profit)
    }

    pub fn roi(&self) -> anyhow::Result<f64> {
        if self.state != TokenState::Sold {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit()?;
        let roi = profit / eth_basis;

        Ok(roi)
    }

    // interval is 1..N
    pub fn profit_at_volume_interval_(&self, interval: usize) -> anyhow::Result<f32> {
        if !self.is_sold_at_time[interval - 1] {
            return Ok(0_f32);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let total_cost = eth_basis * interval + self.tx_gas_cost;
        let profit = if self.amounts_sold[interval - 1] >= total_cost {
            let abs_profit = self.amounts_sold[interval - 1] - total_cost;
            abs_profit.as_u128() as i128
        } else {
            let abs_profit = total_cost - self.amounts_sold[interval - 1];
            -(abs_profit.as_u128() as i128)
        };

        let profit = profit as f64 / 1e18_f64;

        Ok(profit as f32)
    }

    pub fn roi_at_volume_interval(&self, interval: usize) -> anyhow::Result<f32> {
        if !self.is_sold_at_time[interval - 1] {
            return Ok(0_f32);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit_at_volume_interval_(interval)?;
        let roi = profit / eth_basis as f32;

        Ok(roi as f32)
    }

    pub fn profit_at_time_interval_(&self, interval: usize) -> anyhow::Result<f64> {
        if !self.is_sold_at_time[interval - 1] {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let total_cost = eth_basis + self.tx_gas_cost;
        let profit = if self.amount_sold_at_time[interval - 1] >= total_cost {
            let abs_profit = self.amount_sold_at_time[interval - 1] - total_cost;
            abs_profit.as_u128() as i128
        } else {
            let abs_profit = total_cost - self.amount_sold_at_time[interval - 1];
            -(abs_profit.as_u128() as i128)
        };

        let profit = profit as f64 / 1e18_f64;

        Ok(profit)
    }

    pub fn roi_at_time_interval(&self, interval: usize) -> anyhow::Result<f64> {
        if !self.is_sold_at_time[interval - 1] {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit_at_time_interval_(interval)?;
        let roi = profit / eth_basis;

        Ok(roi)
    }

    pub fn lowercase_address(&self) -> String {
        let address_string = address_to_string(self.address);

        return address_string.to_lowercase();
    }
}
