use anyhow::Result;
use ethers::{
    core::types::TxHash,
    providers::{Provider, Ws},
};
use ethers::{prelude::*, utils::keccak256};
use log::info;
use std::sync::Arc;

use super::decode_add_liquidity::decode_add_liquidity_eth_fn;

pub async fn detect_price_update_and_find_users_to_liquidate(
    pending_tx: TxHash,
    client: &Arc<Provider<Ws>>,
) -> Result<()> {
    let add_liquidity_signature = "addLiquidityETH(address,uint,uint,uint,address,uint)";

    // Compute the Keccak-256 hashes of the event signatures
    let add_liquidity_hash = keccak256(add_liquidity_signature.as_bytes())[0..4].to_vec();

    // Print out each new transaction hash
    if let Ok(Some(tx)) = client.get_transaction(pending_tx).await {
        // If the transaction involves a contract interaction, `to` will be Some(address)
        if let Some(_) = tx.to {
            // The `data` field contains the input data for contract interactions
            if !tx.input.0.is_empty() && tx.input.0.len() >= 4 {
                let data = tx.input.0.clone();

                if data.starts_with(&add_liquidity_hash) {
                    // extract address from data ==> forward(address,bytes)
                    let token = decode_add_liquidity_eth_fn(&data.into())?;

                    info!(
                        "detected Add Liquidity tx in mempool for token => {}",
                        token
                    );

                    // TODO - check that token is one we are looking for!
                }
            }
        }
    }
    Ok(())
}
