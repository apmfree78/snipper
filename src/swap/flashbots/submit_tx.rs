use anyhow::anyhow;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::core::rand::thread_rng;
use ethers::signers::Wallet;
use ethers::types::{U256, U64};
use ethers::{
    core::types::transaction::eip2718::TypedTransaction,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{LocalWallet, Signer},
    types::Eip1559TransactionRequest,
};
use ethers_flashbots::{BroadcasterMiddleware, BundleRequest, PendingBundleError, SimulatedBundle};
use log::{error, info};
use std::sync::Arc;
use url::Url;

pub type FlashbotsBroadcaster = BroadcasterMiddleware<Arc<Provider<Ws>>, LocalWallet>;

// ========== HELPER FUNCTIONS ==========
pub fn generate_flashbot_signed_client_with_builders(
    wallet: &Wallet<SigningKey>,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<SignerMiddleware<FlashbotsBroadcaster, Wallet<SigningKey>>> {
    // for multiple builder urls
    static BUILDER_URLS: &[&str] = &[
        "https://builder0x69.io",
        "https://rpc.beaverbuild.org",
        "https://relay.flashbots.net",
        "https://rsync-builder.xyz",
        "https://rpc.titanbuilder.xyz",
        "https://api.blocknative.com/v1/auction",
        "https://mev.api.blxrbdn.com",
        "https://eth-builder.com",
        "https://builder.gmbit.co/rpc",
        "https://buildai.net",
        "https://rpc.payload.de",
        "https://rpc.lightspeedbuilder.info",
        "https://rpc.nfactorial.xyz",
        "https://rpc.lokibuilder.xyz",
    ];
    // this is your ephemeral searcher identity
    let bundle_signer = LocalWallet::new(&mut thread_rng());
    // in production, you could parse a key from an env var
    // let bundle_signer: LocalWallet = LocalWallet::from_str("...")?.with_chain_id(Chain::Mainnet);

    // wrap client in BroadcasterMiddleware for multi-relay
    let client_signed = SignerMiddleware::new(
        BroadcasterMiddleware::new(
            Arc::clone(client),
            BUILDER_URLS
                .iter()
                .map(|url| Url::parse(url).unwrap())
                .collect(),
            Url::parse("https://relay.flashbots.net")?,
            bundle_signer,
        ),
        wallet.clone(),
    );

    Ok(client_signed)
}

pub fn is_flashbot_simulation_success(bundle: &SimulatedBundle) -> bool {
    for (index, transaction) in bundle.transactions.iter().enumerate() {
        if let Some(err) = &transaction.error {
            info!("Transaction {} failed with error: {}", index, err);
            return false;
        }
        if let Some(revert) = &transaction.revert {
            info!("Transaction {} reverted with message: {}", index, revert);
            return false;
        }
    }
    true
}

pub async fn create_flashbot_bundle_with_tx(
    tx_requests: &[Eip1559TransactionRequest],
    signed_client: &SignerMiddleware<FlashbotsBroadcaster, Wallet<SigningKey>>,
    block_number: U64,
) -> anyhow::Result<BundleRequest> {
    // sign the transaction
    let mut signatures = Vec::with_capacity(tx_requests.len());

    for tx in tx_requests.iter() {
        // Convert EIP1559TransactionRequest into a typed transaction
        let typed_tx = TypedTransaction::Eip1559(tx.clone());

        // Sign it. This call is async, so we `.await` it here.
        let sig = signed_client.signer().sign_transaction(&typed_tx).await?;
        signatures.push(sig);
    }

    let mut bundle = BundleRequest::new()
        // .push_transaction(TypedTransaction::Eip1559(tx_request.clone()).rlp_signed(&signature))
        // simulate on next block
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    for (tx, sig) in tx_requests.into_iter().zip(signatures.into_iter()) {
        bundle = bundle.push_transaction(TypedTransaction::Eip1559(tx.clone()).rlp_signed(&sig));
    }

    Ok(bundle)
}

// returns None of simulation fails
pub async fn simulate_flashbot_tx_and_get_gas_used(
    bundle: &BundleRequest,
    signed_client: &SignerMiddleware<FlashbotsBroadcaster, Wallet<SigningKey>>,
) -> anyhow::Result<Option<U256>> {
    info!("Beginning bundle simulation.....");
    let simulated_bundle: SimulatedBundle = signed_client.inner().simulate_bundle(&bundle).await?;
    info!("Simulated bundle: {:?}", simulated_bundle);

    // check if simulation is success
    if !is_flashbot_simulation_success(&simulated_bundle) {
        error!("error simulating single-tx uniswap swap");
        return Ok(None);
    }
    info!("Single Flashbot transaction simulation succeeded.");

    // get gas used
    let gas_used = simulated_bundle
        .transactions
        .get(0)
        .ok_or_else(|| anyhow!("No transaction found in simulation result"))?
        .gas_used;

    Ok(Some(gas_used))
}

pub async fn submit_production_flashbot_tx(
    txs: &[Eip1559TransactionRequest],
    block_number: U64,
    miner_bribe: U256,
    signed_client: &SignerMiddleware<FlashbotsBroadcaster, Wallet<SigningKey>>,
) -> anyhow::Result<()> {
    // re-build the transaction => only difference is max_priority_fee_per_gas = bribe
    let txs_final: Vec<Eip1559TransactionRequest> = txs
        .iter()
        .map(|tx| Eip1559TransactionRequest {
            max_priority_fee_per_gas: Some(miner_bribe),
            ..tx.clone()
        })
        .collect();

    let mut signatures = Vec::with_capacity(txs_final.len());

    for tx in txs_final.iter() {
        // Convert EIP1559TransactionRequest into a typed transaction
        let typed_tx = TypedTransaction::Eip1559(tx.clone());

        // Sign it. This call is async, so we `.await` it here.
        let sig = signed_client.signer().sign_transaction(&typed_tx).await?;
        signatures.push(sig);
    }

    let mut production_bundle = BundleRequest::new()
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    for (tx, sig) in txs_final.into_iter().zip(signatures.into_iter()) {
        production_bundle =
            production_bundle.push_transaction(TypedTransaction::Eip1559(tx).rlp_signed(&sig));
    }

    let results = signed_client
        .inner()
        .send_bundle(&production_bundle)
        .await?;

    // optionally wait for inclusion
    for result in results {
        match result {
            Ok(pending_bundle) => match pending_bundle.await {
                Ok(bundle_hash) => {
                    info!("Single-tx backrun with hash {:?} included!", bundle_hash)
                }
                Err(PendingBundleError::BundleNotIncluded) => {
                    info!("Single-tx backrun was not included in target block.")
                }
                Err(e) => info!("An error occurred: {}", e),
            },
            Err(e) => info!("An error occurred: {}", e),
        }
    }
    Ok(())
}
