use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use ethers::utils::format_units;
use std::sync::Arc;

use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;

pub async fn mock_buying_token_for_weth(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<U256> {
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;

    println!("........................................................");
    let amount_to_buy =
        std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
    println!("buying {} WETH of {}", amount_to_buy, token.name);
    let amount_in = ethers::utils::parse_ether(amount_to_buy)?;

    // calculate amount amount out and gas used
    println!("........................................................");
    let amount_out =
        get_amount_out_uniswap_v2(weth_address, token.address, amount_in, client).await?;

    let amount_out_readable = format_units(amount_out, u32::from(token.decimals))?;
    println!("calculated amount out min {}", amount_out_readable);
    println!("........................................................");
    Ok(amount_out)
}

pub async fn mock_sell_token_for_weth(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<U256> {
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;

    println!("........................................................");
    let amount_to_sell = token.amount_bought;

    //approve swap router to trade toke
    println!("........................................................");
    let amount_out =
        get_amount_out_uniswap_v2(token.address, weth_address, amount_to_sell, client).await?;

    let amount_out_min_readable = format_units(amount_out, 18u32)?;
    println!("calculated amount out min {}", amount_out_min_readable);
    println!("........................................................");

    Ok(amount_out)
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

    // reduce by 2% to account for token volatility
    let amount_out = amount_out * U256::from(98) / U256::from(100);

    Ok(amount_out)
}
