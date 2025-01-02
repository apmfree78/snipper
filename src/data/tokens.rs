use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use log::info;
use std::sync::Arc;

use crate::abi::uniswap_pair::UNISWAP_PAIR;
use crate::token_tx::time_intervals::TIME_ROUNDS;
use crate::token_tx::volume_intervals::VOLUME_ROUNDS;
use crate::utils::type_conversion::address_to_string;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum TokenState {
    #[default]
    NotValidated,
    Validating,
    Validated,
    Buying,
    Bought,
    Selling,
    Sold,
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
}

impl Erc20Token {
    pub async fn get_total_supply(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<U256> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        info!("getting total liquidity");
        let supply = pool.total_supply().call().await?;

        Ok(supply)
    }

    pub fn profit(&self) -> anyhow::Result<f32> {
        if self.eth_recieved_at_sale == U256::zero() {
            return Ok(0_f32);
        }

        let eth_basis =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let eth_basis = ethers::utils::parse_ether(eth_basis)?;

        let total_cost = eth_basis + self.tx_gas_cost;
        let profit = if self.eth_recieved_at_sale >= total_cost {
            let abs_profit = self.eth_recieved_at_sale - total_cost;
            abs_profit.as_u128() as i128
        } else {
            let abs_profit = total_cost - self.eth_recieved_at_sale;
            -(abs_profit.as_u128() as i128)
        };

        let profit = profit as f64 / 1e18_f64;

        Ok(profit as f32)
    }

    pub fn roi(&self) -> anyhow::Result<f32> {
        if self.eth_recieved_at_sale == U256::zero() {
            return Ok(0_f32);
        }
        let eth_basis =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let eth_basis = ethers::utils::parse_ether(eth_basis)?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit()?;
        let roi = profit / eth_basis as f32;

        Ok(roi as f32)
    }

    // interval is 1..N
    pub fn profit_at_volume_interval_(&self, interval: usize) -> anyhow::Result<f32> {
        if self.amounts_sold[interval - 1] == U256::zero() {
            return Ok(0_f32);
        }
        let eth_basis =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let eth_basis = ethers::utils::parse_ether(eth_basis)?;

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
        if self.amounts_sold[interval - 1] == U256::zero() {
            return Ok(0_f32);
        }

        let eth_basis =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let eth_basis = ethers::utils::parse_ether(eth_basis)?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit_at_volume_interval_(interval)?;
        let roi = profit / eth_basis as f32;

        Ok(roi as f32)
    }

    pub fn profit_at_time_interval_(&self, interval: usize) -> anyhow::Result<f32> {
        if self.amount_sold_at_time[interval - 1] == U256::zero() {
            return Ok(0_f32);
        }

        let eth_basis =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let eth_basis = ethers::utils::parse_ether(eth_basis)?;

        let total_cost = eth_basis + self.tx_gas_cost;
        let profit = if self.amount_sold_at_time[interval - 1] >= total_cost {
            let abs_profit = self.amount_sold_at_time[interval - 1] - total_cost;
            abs_profit.as_u128() as i128
        } else {
            let abs_profit = total_cost - self.amount_sold_at_time[interval - 1];
            -(abs_profit.as_u128() as i128)
        };

        let profit = profit as f64 / 1e18_f64;

        Ok(profit as f32)
    }

    pub fn roi_at_time_interval(&self, interval: usize) -> anyhow::Result<f32> {
        if self.amount_sold_at_time[interval - 1] == U256::zero() {
            return Ok(0_f32);
        }

        let eth_basis =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let eth_basis = ethers::utils::parse_ether(eth_basis)?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit_at_time_interval_(interval)?;
        let roi = profit / eth_basis as f32;

        Ok(roi as f32)
    }

    pub fn lowercase_address(&self) -> String {
        let address_string = address_to_string(self.address);

        return address_string.to_lowercase();
    }
}
