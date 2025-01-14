use crate::swap::mainnet::setup::{TxWallet, WalletType};
use futures::lock::Mutex;
use once_cell::sync::Lazy;
use std::sync::Arc;

use ethers::{
    providers::{Middleware, Provider, Ws},
    types::{Address, BlockId, BlockNumber, U256},
};
static NONCE_MAIN: Lazy<Arc<Mutex<U256>>> = Lazy::new(|| Arc::new(Mutex::new(U256::zero())));
static NONCE_TEST: Lazy<Arc<Mutex<U256>>> = Lazy::new(|| Arc::new(Mutex::new(U256::zero())));

pub async fn intialize_nonce(tx_wallet: &TxWallet, wallet_type: WalletType) -> anyhow::Result<()> {
    let nonce_state = match wallet_type {
        WalletType::Main => Arc::clone(&NONCE_MAIN),
        WalletType::Test => Arc::clone(&NONCE_TEST),
    };
    let mut nonce = nonce_state.lock().await;

    let current_nonce = get_wallet_nonce(tx_wallet.sender, &tx_wallet.client).await?;

    *nonce = current_nonce;

    Ok(())
}

pub async fn get_next_nonce(wallet_type: WalletType) -> U256 {
    let nonce_state = match wallet_type {
        WalletType::Main => Arc::clone(&NONCE_MAIN),
        WalletType::Test => Arc::clone(&NONCE_TEST),
    };
    let mut nonce = nonce_state.lock().await;
    let current = *nonce;

    *nonce += U256::one();

    current
}

pub async fn get_wallet_nonce(
    wallet_address: Address,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<U256> {
    let nonce = client
        .get_transaction_count(wallet_address, Some(BlockId::Number(BlockNumber::Latest)))
        .await?;
    Ok(nonce)
}
