use derive_more::Display;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use std::sync::Arc;

use crate::abi::uniswap_pair::UNISWAP_PAIR;
use crate::app_config::{
    CHECK_IF_LIQUIDITY_LOCKED, HIGH_LIQUIDITY_THRESHOLD, LIQUIDITY_PERCENTAGE_LOCKED,
    LOW_LIQUIDITY_THRESHOLD, MEDIUM_LIQUIDITY_THRESHOLD, MIN_LIQUIDITY, MIN_RESERVE_ETH_FACTOR,
    MIN_TRADE_FACTOR, TIME_ROUNDS, VERY_LOW_LIQUIDITY_THRESHOLD,
};
use crate::data::token_state_update::remove_token;
use crate::swap::mainnet::setup::TxType;
use crate::utils::tx::amount_of_token_to_purchase;
use crate::verify::check_token_lock::is_liquidity_locked;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum TokenState {
    #[default]
    NotValidated,
    Validating,
    CannotBuy,
    Validated,
    Locked,
    FullyValidated,
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

    pub source_code: String,
    pub source_code_tokens: u32,

    pub pair_address: Address, // uniswap pair address
    pub is_token_0: bool,

    // token state
    pub is_tradable: bool,
    pub state: TokenState,
    pub liquidity: TokenLiquidity,
    pub amount_bought: U256,
    pub eth_recieved_at_sale: U256,
    pub time_of_purchase: u32,

    //api retry limits
    pub honeypot_checks: u8,
    pub graphql_checks: u8,

    // buy and sell attempts
    pub purchase_attempts: u8,
    pub sell_attempts: u8,

    // total gas cost for buy + sell of token
    pub tx_gas_cost: U256,

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
    pub async fn check_liquidity_is_locked_and_update_state(
        &self,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<bool> {
        //  SYSTEM OVERRIDE
        if !CHECK_IF_LIQUIDITY_LOCKED {
            self.set_state_to_(TokenState::Locked).await;
            return Ok(true);
        }

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

    // pub async fn check_if_token_is_honeypot(&self) -> anyhow::Result<Option<bool>> {
    //     //  SYSTEM OVERRIDE
    //     if !CHECK_IF_HONEYPOT {
    //         return Ok(Some(false));
    //     }
    //
    //     let honeypot_check_count = self.honeypot_check_count().await;
    //
    //     if honeypot_check_count > API_CHECK_LIMIT {
    //         warn!(
    //             "{} exceed honeypot api check limit removing token!",
    //             self.name
    //         );
    //         let _ = remove_token(self.address).await.unwrap();
    //         return Ok(Some(true)); // if canot check assume its a honeypot and remove token
    //     }
    //
    //     let (token_summary, honeypot_result) = match self.is_honeypot().await {
    //         Ok((summary, result)) => (summary, result),
    //         Err(error) => {
    //             error!("could not get honeypot status => {}", error);
    //             self.increment_honeypot_checks().await;
    //             return Ok(None);
    //         }
    //     };
    //
    //     println!("{} token risk is {} ", self.name, token_summary.risk);
    //     if honeypot_result.is_honeypot {
    //         println!("{} is a honeypot scam! Removing...", self.name);
    //         let _ = remove_token(self.address).await.unwrap();
    //     } else {
    //         println!("{} is NOT a honeypot! :)", self.name);
    //     }
    //
    //     // increment api call count
    //     self.increment_honeypot_checks().await;
    //
    //     Ok(Some(honeypot_result.is_honeypot))
    // }

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

        let eth_amount_used_for_purchase = amount_of_token_to_purchase(TxType::Real)?;

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
}
