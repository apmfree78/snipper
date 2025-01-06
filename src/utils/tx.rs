use crate::abi::erc20::ERC20;
use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::app_config::CHAIN;
use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use anyhow::{anyhow, Result};
use ethers::core::k256::ecdsa::SigningKey;
use ethers::types::{Address, Block, BlockId, BlockNumber, Bytes, H256, U256, U64};
use ethers::utils::format_units;
use ethers::{
    providers::{Middleware, Provider, Ws},
    signers::{LocalWallet, Signer, Wallet},
    types::Eip1559TransactionRequest,
};
use rand::Rng;
use std::cmp::min;
use std::env;
use std::str::FromStr;
use std::sync::Arc;

#[derive(PartialEq, Eq)]
pub enum TxSlippage {
    OnePercent,
    TwoPercent,
    FivePercent,
    TenPercent,
    None,
}

// ************************** WALLET ***************************************************
pub fn get_wallet() -> anyhow::Result<Wallet<SigningKey>> {
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY not found in .env file");

    let wallet = LocalWallet::from_str(&private_key)?.with_chain_id(CHAIN);
    Ok(wallet)
}

// ************************** BLOCK ***************************************************
pub async fn get_current_block(client: &Arc<Provider<Ws>>) -> anyhow::Result<(Block<H256>, U64)> {
    // get the latest block
    let block = client
        .get_block(BlockNumber::Latest)
        .await?
        .ok_or_else(|| {
            anyhow!("Could not retrieve the latest block for next_base_fee calculation")
        })?;

    // block number
    let block_number = block
        .number
        .ok_or_else(|| anyhow!("missing block number"))?;

    Ok((block, block_number))
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

// ************************** SWAP ***************************************************
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
        TxSlippage::TenPercent => base_amount_out * U256::from(90) / U256::from(100),
        TxSlippage::FivePercent => base_amount_out * U256::from(95) / U256::from(100),
        TxSlippage::TwoPercent => base_amount_out * U256::from(98) / U256::from(100),
        TxSlippage::OnePercent => base_amount_out * U256::from(99) / U256::from(100),
        TxSlippage::None => base_amount_out,
    };

    Ok(amount_out)
}

pub fn get_token_sell_interval() -> Result<u32> {
    let token_sell_interval_in_secs =
        std::env::var("TOKEN_SELL_INTERVAL").expect("TOKEN_SELL_INTERVAL is not set in .env");
    let token_sell_interval_in_secs: u32 = token_sell_interval_in_secs.parse()?;
    Ok(token_sell_interval_in_secs)
}

pub fn get_transaction_cost_in_eth(
    txs: &[Eip1559TransactionRequest],
    gas_cost: U256,
    next_base_fee: U256,
) -> Result<U256> {
    let total_gas_price = txs
        .iter()
        .map(|tx| min(tx.max_fee_per_gas.unwrap_or_default(), next_base_fee))
        .fold(U256::zero(), |acc, x| acc.saturating_add(x));

    let transaction_cost = gas_cost.checked_mul(total_gas_price).ok_or_else(|| {
        anyhow!("overflow when computing transaction cost (gas_cost * gas_price)")
    })?;

    Ok(transaction_cost)
}

/// Build the calldata for liquidate_account(..)
pub async fn get_swap_exact_eth_for_tokens_calldata(
    token: &Erc20Token,
    wallet_address: Address,
    current_time: u32,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<Bytes> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    let router = UNISWAP_V2_ROUTER::new(uniswap_v2_router_address, client.clone());

    let amount_to_buy = amount_of_token_to_purchase()?;

    let deadline = U256::from(current_time + 50); // add 50 secs

    // calculate amount amount out and gas used
    println!("........................................................");
    let amount_out_min = get_amount_out_uniswap_v2(
        weth_address,
        token.address,
        amount_to_buy,
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
        .expect("Failed to encode swap calldata");
    Ok(calldata)
}

/// Build the calldata for liquidate_account(..)
pub async fn get_swap_exact_tokens_for_eth_calldata(
    token: &Erc20Token,
    wallet_address: Address,
    tokens_to_sell: U256,
    current_time: u32,
    client: &Arc<Provider<Ws>>,
    slippage: TxSlippage,
) -> anyhow::Result<Bytes> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    let router = UNISWAP_V2_ROUTER::new(uniswap_v2_router_address, client.clone());

    println!("selling {} WETH of {}", tokens_to_sell, token.name);

    let deadline = U256::from(current_time + 300); // add 50 secs

    // calculate amount amount out and gas used
    println!("........................................................");
    let amount_out_min = get_amount_out_uniswap_v2(
        token.address,
        weth_address,
        tokens_to_sell,
        slippage,
        client,
    )
    .await?;

    let amount_out_min_readable = format_units(amount_out_min, 18u32)?;
    println!("calculated amount out min {}", amount_out_min_readable);
    println!("........................................................");
    let calldata = router
        .swap_exact_tokens_for_eth(
            tokens_to_sell,
            amount_out_min,
            vec![token.address, weth_address],
            wallet_address,
            deadline,
        )
        .calldata()
        .expect("Failed to encode swap calldata");
    Ok(calldata)
}

pub fn get_approval_calldata(
    token: &Erc20Token,
    amount_to_approve: U256,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<Bytes> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
    let token_contract = ERC20::new(token.address, client.clone());

    let calldata = token_contract
        .approve(uniswap_v2_router_address, amount_to_approve)
        .calldata()
        .expect("Failed to encode approval calldata");
    Ok(calldata)
}

pub fn token_tx_profit_loss(sold_revenue: U256) -> anyhow::Result<String> {
    let bought_amount = amount_of_token_to_purchase()?;

    if sold_revenue > bought_amount {
        let profit = sold_revenue - bought_amount;
        let profit = format_units(profit, "ether")?;
        return Ok(profit);
    } else {
        let loss = bought_amount - sold_revenue;

        let loss = format_units(loss, "ether")?;
        return Ok("-".to_string() + &loss);
    }
}

pub fn amount_of_token_to_purchase() -> anyhow::Result<U256> {
    let amount_to_buy =
        std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
    let amount_in = ethers::utils::parse_ether(amount_to_buy)?;
    // let purchase_amount = format_units(amount_in, "ether")?;
    // println!("buying {} of token", purchase_amount);
    Ok(amount_in)
}
