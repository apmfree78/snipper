use crate::utils::type_conversion::address_to_string;
use ethers::{abi::Address, core::types::U256};

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
}
