use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use crate::utils::tx::{calculate_next_block_base_fee, get_approval_calldata};
use ethers::core::k256::ecdsa::SigningKey;
use ethers::signers::Wallet;
use ethers::types::{Address, Block, BlockId, BlockNumber, H256, U256};
use ethers::{
    core::types::Chain,
    providers::{Middleware, Provider, Ws},
    signers::Signer,
    types::{Eip1559TransactionRequest, NameOrAddress},
};
use log::info;
use std::sync::Arc;

pub async fn prepare_uniswap_swap_tx(
    calldata: ethers::types::Bytes,
    eth_to_send_with_tx: U256,
    block: &Block<H256>,
    wallet: &Wallet<SigningKey>,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<(Eip1559TransactionRequest, U256)> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;

    // compute next_base_fee
    let next_base_fee = calculate_next_block_base_fee(&block)?;

    // add small buffer
    let buffer = next_base_fee / 20; // 5% buffer
    let adjusted_max_fee = next_base_fee + buffer;

    // get transaction nonce
    let nonce = client
        .get_transaction_count(wallet.address(), Some(BlockId::Number(BlockNumber::Latest)))
        .await?;
    info!("wallet nonce => {}", nonce);

    // build the initial EIP-1559 transaction (no priority fee yet)
    let uniswap_swap_tx = Eip1559TransactionRequest {
        chain_id: Some(Chain::Mainnet.into()),
        max_priority_fee_per_gas: Some(U256::zero()), // initially zero, we’ll refine after simulation
        max_fee_per_gas: Some(adjusted_max_fee),
        gas: Some(U256::from(300_000u64)),
        to: Some(NameOrAddress::Address(uniswap_v2_router_address)),
        data: Some(calldata),
        nonce: Some(nonce),
        value: Some(eth_to_send_with_tx),
        ..Default::default()
    };

    //*********************
    Ok((uniswap_swap_tx, adjusted_max_fee))
}

// ========== APPROVAL TX ==========
pub async fn prepare_token_approval_tx(
    token: &Erc20Token,
    amount_to_approve: U256,
    block: &Block<H256>,
    wallet: &Wallet<SigningKey>,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<Eip1559TransactionRequest> {
    // 1) Encode approval data
    let calldata = get_approval_calldata(token, amount_to_approve, client)?;

    // 2) Grab next_base_fee (reuse your existing function)
    let next_base_fee = calculate_next_block_base_fee(block)?;

    // 3) Add small buffer to max fee
    let buffer = next_base_fee / 20;
    let adjusted_max_fee = next_base_fee + buffer;

    // 4) Get nonce
    let nonce = client
        .get_transaction_count(wallet.address(), Some(BlockId::Number(BlockNumber::Latest)))
        .await?;

    info!("Approval TX nonce => {}", nonce);

    // 5) Build the EIP-1559 transaction
    let approval_tx = Eip1559TransactionRequest {
        chain_id: Some(Chain::Mainnet.into()),
        max_priority_fee_per_gas: Some(U256::zero()),
        max_fee_per_gas: Some(adjusted_max_fee),
        gas: Some(U256::from(100_000u64)), // likely less for simple approve
        to: Some(NameOrAddress::Address(token.address)),
        data: Some(calldata),
        nonce: Some(nonce),
        value: Some(U256::zero()),
        ..Default::default()
    };

    Ok(approval_tx)
}
