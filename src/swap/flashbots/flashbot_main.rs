use crate::abi::erc20::ERC20;
use crate::data::nonce::get_next_nonce;
use crate::data::tokens::Erc20Token;
use crate::swap::flashbots::submit_tx::{
    create_flashbot_bundle_with_tx, generate_flashbot_signed_client_with_builders,
    simulate_flashbot_tx_and_get_gas_used, submit_production_flashbot_tx,
};
use crate::swap::prepare_tx::prepare_uniswap_swap_tx;
use crate::utils::tx::{
    amount_of_token_to_purchase, get_current_block, get_swap_exact_eth_for_tokens_calldata,
    get_swap_exact_tokens_for_eth_calldata, get_transaction_cost_in_eth, get_wallet, TxSlippage,
};
use anyhow::Result;
use ethers::abi::Address;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::providers::{Provider, Ws};
use ethers::signers::{Signer, Wallet};
use ethers::types::{Eip1559TransactionRequest, U256};
use ethers::utils::format_units;
use std::sync::Arc;

use crate::swap::prepare_tx::prepare_token_approval_tx;

#[derive(PartialEq, Eq)]
pub enum FlashbotsMode {
    RunSimulation,
    RunProduction,
    RunSimulationAndProduction,
}

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

    println!("getting amount of token to sell (balance)...");
    let token_balance = get_wallet_token_balance(token.address, &wallet, client).await?;
    println!("getting calldata for approval...");

    // 4) Get nonce
    let mut nonce = get_next_nonce().await;
    println!("nonce for approval tx => {}", nonce);

    println!("preparing approval tx...");
    let approval_tx = prepare_token_approval_tx(&token, token_balance, &block, nonce, client)?;

    println!("getting calldata for swap...");
    // encode the call data
    let token_swap_calldata = get_swap_exact_tokens_for_eth_calldata(
        &token,
        wallet.address(),
        token_balance,
        block.timestamp.as_u32(),
        client,
        TxSlippage::TwoPercent,
    )
    .await?;

    println!("iterate nonce for swap tx...");
    nonce += U256::from(1);
    println!("nonce for swap tx => {}", nonce);

    println!("preparing swap tx...");
    let (uniswap_swap_tx, max_gas_fee) =
        prepare_uniswap_swap_tx(token_swap_calldata, U256::zero(), &block, nonce)?;

    // let txs = vec![approval_tx, uniswap_swap_tx];
    let txs = vec![approval_tx, uniswap_swap_tx];

    println!("submitting flashbot tx...");
    submit_flashbots_tx(
        txs,
        client,
        &wallet,
        max_gas_fee,
        block_number,
        FlashbotsMode::RunSimulationAndProduction,
    )
    .await?;

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

    println!("getting block data...");
    let (block, block_number) = get_current_block(client).await?;

    println!("getting calldata...");
    // FOR swap exact eth ONLY
    // encode the call data
    let calldata = get_swap_exact_eth_for_tokens_calldata(
        &token,
        wallet.address(),
        block.timestamp.as_u32(),
        client,
    )
    .await?;

    println!("getting amount of token to purchase...");
    // FOR swap exact eth ONLY
    let eth_to_send_with_tx = amount_of_token_to_purchase()?;

    let nonce = get_next_nonce().await;
    println!("nonce for purchase tx => {}", nonce);

    println!("getting amount of token to purchase...");
    let (uniswap_swap_tx, max_gas_fee) =
        prepare_uniswap_swap_tx(calldata, eth_to_send_with_tx, &block, nonce)?;

    let txs = vec![uniswap_swap_tx];

    println!("submitting flashbot tx...");
    submit_flashbots_tx(
        txs,
        client,
        &wallet,
        max_gas_fee,
        block_number,
        FlashbotsMode::RunSimulationAndProduction,
    )
    .await?;

    Ok(())
}

pub async fn submit_flashbots_tx(
    txs: Vec<Eip1559TransactionRequest>,
    client: &Arc<Provider<Ws>>,
    wallet: &Wallet<SigningKey>,
    max_gas_fee: U256,
    block_number: ethers::types::U64,
    mode: FlashbotsMode, // run simulation and or production?
) -> Result<()> {
    // ============================================================
    // 2) FLASHBOTS MIDDLEWARE
    // ============================================================

    println!("signing tx...");
    let signed_client = generate_flashbot_signed_client_with_builders(&wallet, client)?;

    println!("creating flashbot tx...");
    // ============================================================
    // 3) BUILD A BUNDLE WITH ONLY ONE TX
    // ============================================================
    // refactor to accept vec?
    let bundle = create_flashbot_bundle_with_tx(&txs, &signed_client, block_number).await?;

    if mode == FlashbotsMode::RunSimulation || mode == FlashbotsMode::RunSimulationAndProduction {
        println!("simulating tx...");
        // ============================================================
        // 4) SIMULATE
        // ============================================================
        let gas_used = simulate_flashbot_tx_and_get_gas_used(&bundle, &signed_client).await?;

        println!("get tx cost...");
        // compute transaction cost
        let transaction_cost = match gas_used {
            Some(gas) => get_transaction_cost_in_eth(&txs, gas, max_gas_fee)?,
            None => return Ok(()), // simulaton failed
        };

        // ============================================================
        // 5) DETERMINE MINER BRIBE AND SET AS PRIORITY FEE
        // ============================================================
        // in your requirement => "priority gas fee = 10% transaction cost"
        let tx_cost = format_units(transaction_cost, "ether")?;
        println!("tx cost => {}", tx_cost);
    }
    // ============================================================
    // 6) SUBMIT FOR PRODUCTION
    // ============================================================

    let gas_fee = format_units(max_gas_fee, "gwei")?;
    println!("gas fee is => {} gwei", gas_fee);

    // let bribe = max_gas_fee * U256::one() / U256::from(5);
    let bribe = max_gas_fee * U256::from(98) / U256::from(100);
    let miner_bribe_readable = format_units(bribe, "gwei")?;
    println!(
        "miner bribe => 20% of transaction cost => {} gwei",
        miner_bribe_readable
    );

    if mode == FlashbotsMode::RunProduction || mode == FlashbotsMode::RunSimulationAndProduction {
        submit_production_flashbot_tx(&txs, block_number, bribe, &signed_client).await?;
    }

    Ok(())
}

async fn get_wallet_token_balance(
    token_address: Address,
    wallet: &Wallet<SigningKey>,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<U256> {
    let token_contract = ERC20::new(token_address, client.clone());
    let wallet_address = wallet.address();

    let token_balance = token_contract.balance_of(wallet_address).await?;

    Ok(token_balance)
}
