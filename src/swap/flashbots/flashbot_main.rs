use crate::data::tokens::Erc20Token;
use crate::swap::flashbots::prepare_tx::prepare_uniswap_swap_tx;
use crate::swap::flashbots::submit_tx::{
    create_flashbot_bundle_with_tx, generate_flashbot_signed_client_with_builders,
    simulate_flashbot_tx_and_get_gas_used, submit_production_flashbot_tx,
};
use crate::utils::tx::{
    amount_of_token_to_purchase, get_current_block, get_swap_exact_eth_for_tokens_calldata,
    get_swap_exact_tokens_for_eth_calldata, get_transaction_cost_in_eth, get_wallet,
    get_wallet_token_balance,
};
use anyhow::Result;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::providers::{Provider, Ws};
use ethers::signers::Wallet;
use ethers::types::{Eip1559TransactionRequest, U256};
use ethers::utils::format_units;
use log::debug;
use std::sync::Arc;

use super::prepare_tx::prepare_token_approval_tx;

pub async fn prepare_and_submit_flashbot_token_sell_tx(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> Result<()> {
    // ============================================================
    // 1) PREPARE
    // ============================================================

    // load private key
    let wallet = get_wallet()?;

    let (block, block_number) = get_current_block(client).await?;

    let token_balance = get_wallet_token_balance(token.address, &wallet, client).await?;
    let approval_tx =
        prepare_token_approval_tx(&token, token_balance, &block, &wallet, client).await?;

    // encode the call data
    let token_swap_calldata = get_swap_exact_tokens_for_eth_calldata(
        &token,
        block.timestamp.as_u32(),
        token_balance,
        client,
    )
    .await?;

    let (uniswap_swap_tx, max_gas_fee) =
        prepare_uniswap_swap_tx(token_swap_calldata, U256::zero(), &block, &wallet, client).await?;

    let txs = vec![approval_tx, uniswap_swap_tx];

    submit_flashbots_tx(txs, client, &wallet, max_gas_fee, block_number).await?;

    Ok(())
}

pub async fn prepare_and_submit_flashbot_token_purchase_tx(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> Result<()> {
    // ============================================================
    // 1) PREPARE
    // ============================================================

    // load private key
    let wallet = get_wallet()?;

    let (block, block_number) = get_current_block(client).await?;

    // FOR swap exact eth ONLY
    // encode the call data
    let calldata =
        get_swap_exact_eth_for_tokens_calldata(&token, block.timestamp.as_u32(), client).await?;

    // FOR swap exact eth ONLY
    let eth_to_send_with_tx = amount_of_token_to_purchase()?;

    let (uniswap_swap_tx, max_gas_fee) =
        prepare_uniswap_swap_tx(calldata, eth_to_send_with_tx, &block, &wallet, client).await?;

    let txs = vec![uniswap_swap_tx];

    submit_flashbots_tx(txs, client, &wallet, max_gas_fee, block_number).await?;

    Ok(())
}

pub async fn submit_flashbots_tx(
    txs: Vec<Eip1559TransactionRequest>,
    client: &Arc<Provider<Ws>>,
    wallet: &Wallet<SigningKey>,
    max_gas_fee: U256,
    block_number: ethers::types::U64,
) -> Result<()> {
    // ============================================================
    // 2) FLASHBOTS MIDDLEWARE
    // ============================================================

    let signed_client = generate_flashbot_signed_client_with_builders(&wallet, client)?;

    // ============================================================
    // 3) BUILD A BUNDLE WITH ONLY ONE TX
    // ============================================================
    // refactor to accept vec?
    let bundle = create_flashbot_bundle_with_tx(&txs, &signed_client, block_number).await?;

    // ============================================================
    // 4) SIMULATE
    // ============================================================
    let gas_used = simulate_flashbot_tx_and_get_gas_used(&bundle, &signed_client).await?;

    // compute transaction cost
    let transaction_cost = match gas_used {
        Some(gas) => get_transaction_cost_in_eth(&txs, gas, max_gas_fee)?,
        None => return Ok(()), // simulaton failed
    };

    // ============================================================
    // 5) DETERMINE MINER BRIBE AND SET AS PRIORITY FEE
    // ============================================================
    // in your requirement => "priority gas fee = 10% transaction cost"
    let miner_bribe = transaction_cost * U256::one() / U256::from(10);
    let miner_bribe_readable = format_units(miner_bribe, 18u32)?;
    debug!(
        "miner bribe => half of transaction cost => {} ETH",
        miner_bribe_readable
    );

    // ============================================================
    // 6) SUBMIT FOR PRODUCTION
    // ============================================================

    submit_production_flashbot_tx(&txs, block_number, miner_bribe, &signed_client).await?;
    Ok(())
}
