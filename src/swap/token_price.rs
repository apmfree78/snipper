use ethers::providers::{Provider, Ws};
use ethers::types::U256;
use log::info;
use std::sync::Arc;

use crate::abi::uniswap_pair::UNISWAP_PAIR;
use crate::data::tokens::Erc20Token;

// pub async fn get_token_price(
//     token: &Erc20Token,
//     client: &Arc<Provider<Ws>>,
// ) -> anyhow::Result<f64> {
//     let pool = UNISWAP_PAIR::new(token.pair_address, client.clone());
//     let weth_address: Address = CONTRACT.get_address().weth.parse()?;
//
//     // Call slot0 on the pool
//     let (sqrt_price_x96, _, _, _, _, _, _) = pool.slot_0().call().await?;
//
//     // Determine which token is WETH and which is your target token
//     let (token0_address, token1_address) = if token.is_token_0 {
//         (token.address, weth_address)
//     } else {
//         (weth_address, token.address)
//     };
//
//     // Fetch token decimals if needed:
//     let token0_contract = ERC20::new(token0_address, client.clone());
//     let token1_contract = ERC20::new(token1_address, client.clone());
//     let decimals0 = token0_contract.decimals().call().await?;
//     let decimals1 = token1_contract.decimals().call().await?;
//
//     // Check which is WETH and compute price
//     let sqrt_price_f64 = sqrt_price_x96.as_u128() as f64;
//     let base = f64::powi(2.0, 192);
//
//     let price = if !token.is_token_0 {
//         // price = (p^2) / 2^192
//         let raw_price = (sqrt_price_f64 * sqrt_price_f64) / base;
//         // Adjust for decimals if needed:
//         // token1 in WETH
//         // (If token1 has `d1` decimals and WETH has `d0` = 18)
//         // price_per_token_in_weth = raw_price * 10^(d1 - d0)
//         raw_price * f64::powi(10.0, decimals1 as i32 - 18)
//     } else {
//         // token1 is WETH
//         // price(token0 in WETH) = 2^192 / (p^2)
//         let raw_price = base / (sqrt_price_f64 * sqrt_price_f64);
//         // Adjust for decimals:
//         raw_price * f64::powi(10.0, decimals0 as i32 - 18)
//     };
//
//     Ok(price)
// }

pub async fn get_token_weth_total_supply(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<U256> {
    let pool = UNISWAP_PAIR::new(token.pair_address, client.clone());

    info!("getting total liquidity");
    let supply = pool.total_supply().call().await?;

    Ok(supply)
}
