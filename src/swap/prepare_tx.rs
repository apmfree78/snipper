use crate::app_config::CHAIN;
use crate::data::contracts::CONTRACT;
use crate::data::gas::GasFeeType;
use crate::data::tokens::Erc20Token;
use crate::utils::tx::{calculate_next_block_base_fee, get_approval_calldata};
use ethers::types::{Address, Block, H256, U256};
use ethers::{
    core::types::Chain,
    providers::{Provider, Ws},
    types::{Eip1559TransactionRequest, NameOrAddress},
};
use std::sync::Arc;

pub fn prepare_uniswap_swap_tx(
    calldata: ethers::types::Bytes,
    eth_to_send_with_tx: U256,
    block: &Block<H256>,
    nonce: U256,
    gas_fee_type: GasFeeType,
) -> anyhow::Result<(Eip1559TransactionRequest, U256)> {
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;

    let (adjusted_max_fee, max_priority_fee) = get_gas_and_priority_fees(block, gas_fee_type)?;

    println!("preparing tx for {}", CHAIN);
    // build the initial EIP-1559 transaction (no priority fee yet)
    let uniswap_swap_tx = Eip1559TransactionRequest {
        chain_id: Some(CHAIN.into()),
        max_priority_fee_per_gas: Some(max_priority_fee), // initially zero, we’ll refine after simulation
        max_fee_per_gas: Some(adjusted_max_fee),
        gas: Some(U256::from(500_000u64)),
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
pub fn prepare_token_approval_tx(
    token: &Erc20Token,
    amount_to_approve: U256,
    block: &Block<H256>,
    nonce: U256,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<Eip1559TransactionRequest> {
    // 1) Encode approval data
    let calldata = get_approval_calldata(token, amount_to_approve, client)?;

    let (adjusted_max_fee, max_priority_fee) =
        get_gas_and_priority_fees(block, GasFeeType::Standard)?;

    // 5) Build the EIP-1559 transaction
    let approval_tx = Eip1559TransactionRequest {
        chain_id: Some(CHAIN.into()),
        max_priority_fee_per_gas: Some(max_priority_fee),
        max_fee_per_gas: Some(adjusted_max_fee),
        gas: Some(U256::from(150_000u64)), // likely less for simple approve
        to: Some(NameOrAddress::Address(token.address)),
        data: Some(calldata),
        nonce: Some(nonce),
        value: Some(U256::zero()),
        ..Default::default()
    };

    Ok(approval_tx)
}

fn get_gas_and_priority_fees(
    block: &Block<H256>,
    gas_fee_type: GasFeeType,
) -> anyhow::Result<(U256, U256)> {
    if CHAIN == Chain::Base {
        if gas_fee_type == GasFeeType::Standard {
            let point_one_gwei_in_wei = U256::from(100_000_000u64);
            let priority_fee = point_one_gwei_in_wei;
            let max_fee = point_one_gwei_in_wei * U256::from(15) / U256::from(10);
            Ok((max_fee, priority_fee))
        } else {
            let priority_fee = U256::from(1_000_000_000u64);
            let base_fee = block.base_fee_per_gas.unwrap_or_default();
            let max_fee = base_fee * 2 + priority_fee;
            Ok((max_fee, priority_fee))
        }
    } else {
        // 2) Grab next_base_fee (reuse your existing function)
        let next_base_fee = calculate_next_block_base_fee(block)?;

        let ten_gwei_in_wei = U256::from(10_000_000_000u64);
        // 3) Add small buffer to max fee
        let buffer = next_base_fee / U256::from(7);
        let max_fee = next_base_fee + buffer;
        let priority_fee = if max_fee < ten_gwei_in_wei {
            max_fee
        } else {
            ten_gwei_in_wei
        };

        Ok((max_fee, priority_fee))
    }
}
