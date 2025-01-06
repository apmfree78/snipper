use anyhow::Result;
use ethers::core::types::TxHash;
use ethers::{prelude::*, utils::keccak256};
use log::{error, info, warn};
use std::sync::Arc;

use crate::data::{token_data::get_token, tokens::TokenState};
use crate::swap::mainnet::setup::TxWallet;

use super::decode_add_liquidity::decode_add_liquidity_eth_fn;

pub async fn detect_token_add_liquidity_and_validate(
    pending_tx: TxHash,
    tx_wallet: &Arc<TxWallet>,
    current_time: u32,
) -> Result<()> {
    let add_liquidity_signature = "addLiquidityETH(address,uint,uint,uint,address,uint)";

    // Compute the Keccak-256 hashes of the event signatures
    let add_liquidity_hash = keccak256(add_liquidity_signature.as_bytes())[0..4].to_vec();

    // Print out each new transaction hash
    if let Ok(Some(tx)) = tx_wallet.client.get_transaction(pending_tx).await {
        // If the transaction involves a contract interaction, `to` will be Some(address)
        if let Some(_) = tx.to {
            // The `data` field contains the input data for contract interactions
            if !tx.input.0.is_empty() && tx.input.0.len() >= 4 {
                let data = tx.input.0.clone();

                if data.starts_with(&add_liquidity_hash) {
                    info!("found add liquidity tx => {:?}", tx);
                    // extract address from data ==> forward(address,bytes)
                    let token_address = decode_add_liquidity_eth_fn(&data)?;

                    info!(
                        "detected Add Liquidity tx in mempool for token => {}",
                        token_address
                    );

                    // check that token is one we are looking for!
                    let _result = get_token(token_address).await;

                    // match result {
                    //     Some(token) => {
                    //         // validate token
                    //         if token.state == TokenState::NotValidated {
                    //             let spawn_token = token.clone();
                    //             let spawn_tx = tx.clone();
                    //             let spawn_tx_wallet = Arc::clone(tx_wallet);
                    //             tokio::spawn(async move {
                    //                 info!("validating {} token from mempool!", spawn_token.name);
                    //                 if let Err(error) = validate_token_from_mempool_and_buy(
                    //                     &spawn_token,
                    //                     &spawn_tx,
                    //                     &spawn_tx_wallet,
                    //                     current_time,
                    //                 )
                    //                 .await
                    //                 {
                    //                     error!(
                    //                         "could not validate_token_from_mempool_and_buy => {}",
                    //                         error
                    //                     );
                    //                 }
                    //             });
                    //         };
                    //     }
                    //     None => warn!("this is not the token you're looking for"),
                    // }
                }
            }
        }
    }
    Ok(())
}
