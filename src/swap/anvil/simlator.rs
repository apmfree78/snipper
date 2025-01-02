use crate::data::contracts::CHAIN;
use anyhow::Result;
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{Signer, Wallet},
    types::Address,
    utils::{Anvil, AnvilInstance},
};
use std::sync::Arc;

pub const STARTING_BALANCE: f64 = 1000.0;

pub struct AnvilSimulator {
    pub signed_client: Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>,
    pub client: Arc<Provider<Ws>>,
    pub anvil: AnvilInstance,
    pub sender: Address,
}

impl AnvilSimulator {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        // Main network provider   // Configure Anvil with forking
        let anvil = Anvil::new()
            // .args(["--no-storage-caching", "--code-size-limit", "2048"])
            .fork(rpc_url) // URL of your Geth node
            .chain_id(CHAIN)
            .spawn();

        // setup mock sender
        let from_address: Address = anvil.addresses()[0];
        let private_keys = anvil.keys();
        let from_private_key = private_keys[0].clone();

        // Connect to Anvil
        let anvil_ws_url = anvil.ws_endpoint();
        let provider = Provider::<Ws>::connect(anvil_ws_url).await?;
        let client = Arc::new(provider.clone());

        // Create a wallet with the private key
        let wallet = Wallet::from(from_private_key).with_chain_id(CHAIN);

        // Create the SignerMiddleware
        let signed_client = Arc::new(SignerMiddleware::new(provider, wallet));

        signed_client
            .provider()
            .request::<_, ()>(
                "anvil_setBalance",
                [
                    format!("{:#x}", from_address),
                    "0x3635c9adc5dea00000".to_string(), //100 ETH
                ],
            )
            .await?;

        let simulator = Self {
            signed_client,
            client,
            anvil,
            sender: from_address,
        };

        // simulator.prepare_account().await?;

        Ok(simulator)
    }
}
