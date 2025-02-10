use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use std::sync::Arc;

use super::token_data::ERC20Token;
use crate::abi::erc20::ERC20;
use crate::abi::uniswap_pair::UNISWAP_PAIR;
use crate::app_config::{MIN_LIQUIDITY, MIN_RESERVE_ETH_FACTOR, MIN_TRADE_FACTOR};
use crate::swap::mainnet::setup::TxType;
use crate::utils::tx::amount_of_token_to_purchase;

impl ERC20Token {
    /// return total number of LP (liquidity pool) tokens on uniswap v2
    pub async fn get_total_liquidity_token_supply(
        &self,
        client: &Arc<Provider<Ws>>,
    ) -> anyhow::Result<U256> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        // info!("getting total liquidity");
        let supply = pool.total_supply().call().await?;

        Ok(supply)
    }

    /// return total number of tokens minted for contract
    pub async fn get_total_token_supply(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<U256> {
        let token_contract = ERC20::new(self.address, client.clone());

        // info!("getting total liquidity");
        let supply = token_contract.total_supply().call().await?;

        Ok(supply)
    }

    ///  check if token has enough liquidity base on preset threshold
    pub async fn has_enough_liquidity(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<bool> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        let (reserve0, reserve1, _) = pool.get_reserves().call().await?;

        let eth_supply = if self.is_token_0 { reserve1 } else { reserve0 };

        // let eth = eth_supply as f64 / 1e18_f64;

        Ok(eth_supply >= MIN_LIQUIDITY)
    }

    pub async fn get_liquidity(&self, client: &Arc<Provider<Ws>>) -> anyhow::Result<f64> {
        let pool = UNISWAP_PAIR::new(self.pair_address, client.clone());

        let (reserve0, reserve1, _) = pool.get_reserves().call().await?;

        let eth_supply = if self.is_token_0 { reserve1 } else { reserve0 };

        let eth_supply = eth_supply as f64 / 1e18_f64;

        Ok(eth_supply)
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
