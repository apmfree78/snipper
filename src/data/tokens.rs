use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use ethers::utils::format_units;
use std::sync::Arc;

use crate::data::contracts::CONTRACT;
use crate::utils::type_conversion::address_to_string;

#[derive(Clone, Default, Debug)]
pub struct Erc20Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub address: Address,
    pub pair_address: Address,
    pub is_tradable: bool,
    pub is_validated: bool,
    pub is_validating: bool,
    pub is_token_0: bool,
    pub done_buying: bool,
    pub amount_bought: U256,
    pub eth_recieved_at_sale: U256,
    pub time_of_purchase: u32,
    pub tx_gas_cost: U256,
}

impl Erc20Token {
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

    pub fn lowercase_address(&self) -> String {
        let address_string = address_to_string(self.address);

        return address_string.to_lowercase();
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
        let amount_out =
            get_amount_out_uniswap_v2(weth_address, self.address, amount_in, client).await?;

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
        let amount_out =
            get_amount_out_uniswap_v2(self.address, weth_address, amount_to_sell, client).await?;

        let amount_out_min_readable = format_units(amount_out, 18u32)?;
        println!("sold {} for {} eth", self.name, amount_out_min_readable);
        println!("........................................................");

        Ok(amount_out)
    }
}

pub async fn get_amount_out_uniswap_v2(
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<U256> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
    let router = UNISWAP_V2_ROUTER::new(uniswap_v2_router_address, client.clone());

    let amounts = router
        .get_amounts_out(amount_in, vec![token_in, token_out])
        .call()
        .await?;

    let amount_out = amounts[amounts.len() - 1];

    // NO 2% volatility reduction for mock purchases
    // reduce by 2% to account for token volatility
    // let amount_out = amount_out * U256::from(98) / U256::from(100);

    Ok(amount_out)
}
