use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use crate::utils::type_conversion::u256_to_f64;
use anyhow::{anyhow, Result};
use ethers::types::{Address, Block, Bytes, H256, U256};
use ethers::utils::format_units;
use ethers::{
    providers::{Provider, Ws},
    types::Eip1559TransactionRequest,
};
use rand::Rng;
use std::cmp::min;
use std::sync::Arc;

#[derive(PartialEq, Eq)]
pub enum TxSlippage {
    OnePercent,
    TwoPercent,
    None,
}

pub async fn get_amount_out_uniswap_v2(
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    slippage_tolerance: TxSlippage,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<U256> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
    let router = UNISWAP_V2_ROUTER::new(uniswap_v2_router_address, client.clone());

    let amounts = router
        .get_amounts_out(amount_in, vec![token_in, token_out])
        .call()
        .await?;
    let base_amount_out = amounts[amounts.len() - 1];

    let amount_out = match slippage_tolerance {
        TxSlippage::TwoPercent => base_amount_out * U256::from(98) / U256::from(100),
        TxSlippage::OnePercent => base_amount_out * U256::from(99) / U256::from(100),
        TxSlippage::None => base_amount_out,
    };

    Ok(amount_out)
}

pub fn get_transaction_cost_in_eth(
    tx: &Eip1559TransactionRequest,
    gas_cost: u64,
    next_base_fee: U256,
) -> Result<f64> {
    let gas_price = min(tx.max_fee_per_gas.unwrap_or_default(), next_base_fee);
    let gas_cost_u256 = U256::from(gas_cost);

    let transaction_cost = gas_cost_u256.checked_mul(gas_price).ok_or_else(|| {
        anyhow!("overflow when computing transaction cost (gas_cost * gas_price)")
    })?;

    let wei_to_eth = 10_u64.pow(18) as f64;
    let transaction_cost = u256_to_f64(transaction_cost).unwrap_or_default();
    let transaction_cost_eth = transaction_cost / wei_to_eth;
    Ok(transaction_cost_eth)
}

/// Calculate the next block base fee with minor randomness
pub fn calculate_next_block_base_fee(block: &Block<H256>) -> Result<U256> {
    let base_fee = block
        .base_fee_per_gas
        .ok_or_else(|| anyhow!("Block missing base fee per gas"))?;

    let gas_used = block.gas_used;
    let mut target_gas_used = block.gas_limit / 2;
    if target_gas_used.is_zero() {
        target_gas_used = U256::one();
    }

    let new_base_fee = if gas_used > target_gas_used {
        base_fee + ((base_fee * (gas_used - target_gas_used)) / target_gas_used) / U256::from(8u64)
    } else {
        base_fee - ((base_fee * (target_gas_used - gas_used)) / target_gas_used) / U256::from(8u64)
    };

    let seed = rand::thread_rng().gen_range(0..9);
    Ok(new_base_fee + seed)
}

/// Build the calldata for liquidate_account(..)
pub async fn get_swap_exact_eth_for_tokens_calldata(
    token: &Erc20Token,
    current_time: u32,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<Bytes> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    let router = UNISWAP_V2_ROUTER::new(uniswap_v2_router_address, client.clone());

    let amount_to_buy =
        std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
    println!("buying {} WETH of {}", amount_to_buy, token.name);
    let amount_in = ethers::utils::parse_ether(amount_to_buy.clone())?;

    let deadline = U256::from(current_time + 50); // add 50 secs

    let wallet_address =
        std::env::var("WALLET_ADDRESS").expect("WALLET_ADDRESS is not set in .env");
    let wallet_address: Address = wallet_address.parse()?;

    // calculate amount amount out and gas used
    println!("........................................................");
    let amount_out_min = get_amount_out_uniswap_v2(
        weth_address,
        token.address,
        amount_in,
        TxSlippage::TwoPercent,
        client,
    )
    .await?;

    let amount_out_min_readable = format_units(amount_out_min, 18u32)?;
    println!("calculated amount out min {}", amount_out_min_readable);
    println!("........................................................");
    let calldata = router
        .swap_exact_eth_for_tokens(
            amount_out_min,
            vec![weth_address, token.address],
            wallet_address,
            deadline,
        )
        .calldata()
        .expect("Failed to encode");
    Ok(calldata)
}

pub fn amount_of_token_to_purchase() -> anyhow::Result<U256> {
    let amount_to_buy =
        std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
    let amount_in = ethers::utils::parse_ether(amount_to_buy.clone())?;
    Ok(amount_in)
}
