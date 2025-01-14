use crate::{
    data::{contracts::CONTRACT, nonce::intialize_nonce},
    utils::tx::{get_test_wallet, get_wallet},
};
use anyhow::Result;
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Provider, Ws},
    signers::{Signer, Wallet},
    types::Address,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletType {
    Main,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxType {
    Real,
    Test,
}

#[derive(Clone)]
pub struct TxWallet {
    pub signed_client: Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>,
    pub client: Arc<Provider<Ws>>,
    pub wallet: Wallet<SigningKey>,
    pub sender: Address,
    pub type_of: WalletType,
}

impl TxWallet {
    pub async fn new(wallet_type: WalletType) -> Result<Self> {
        // setup websocket connect to eth node
        // let ws_url = CONTRACT.get_address().ws_url.clone();
        // TODO - switch to ws_url once eth node up
        let ws_url = CONTRACT.get_address().alchemy_url.clone();
        let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
        let client = Arc::new(provider.clone());

        // wallet config and address
        let wallet = if wallet_type == WalletType::Main {
            get_wallet()?
        } else {
            get_test_wallet()?
        };

        let sender = wallet.address();

        // setup signed client
        let signer_middleware = SignerMiddleware::new(provider, wallet.clone());

        let signed_client = Arc::new(signer_middleware);

        let transaction_wallet = Self {
            signed_client,
            client: client.clone(),
            wallet,
            sender,
            type_of: wallet_type.clone(),
        };

        // setup global nonce
        intialize_nonce(&transaction_wallet, wallet_type).await?;

        // let starting_balance = simulator.get_wallet_eth_balance().await?;
        //
        // simulator = Self {
        //     starting_eth_balance: starting_balance,
        //     ..simulator
        // };

        Ok(transaction_wallet)
    }
}
