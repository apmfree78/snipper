use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use crate::utils::tx::{
    amount_of_token_to_purchase, calculate_next_block_base_fee,
    get_swap_exact_eth_for_tokens_calldata, get_transaction_cost_in_eth,
};
use crate::utils::type_conversion::f64_to_u256;
use anyhow::{anyhow, Result};
use ethers::core::rand::thread_rng;
use ethers::types::{Address, BlockId, BlockNumber, U256};
use ethers::{
    core::types::{transaction::eip2718::TypedTransaction, Chain},
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{LocalWallet, Signer},
    types::{Eip1559TransactionRequest, NameOrAddress},
};
use ethers_flashbots::{BroadcasterMiddleware, BundleRequest, PendingBundleError, SimulatedBundle};
use log::{debug, error, info};
use std::str::FromStr;
use std::{env, sync::Arc};
use url::Url;

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

// TODO - set value() to submit ETH with transaction
pub async fn submit_single_flashbots_tx(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> Result<()> {
    // ============================================================
    // 1) PREPARE
    // ============================================================
    let uniswap_v2_router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;

    // get the latest block
    let block = client
        .get_block(BlockNumber::Latest)
        .await?
        .ok_or_else(|| {
            anyhow!("Could not retrieve the latest block for next_base_fee calculation")
        })?;

    // load private key
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY not found in .env file");
    let wallet = LocalWallet::from_str(&private_key)?.with_chain_id(Chain::Mainnet);

    // block number
    let block_number = block
        .number
        .ok_or_else(|| anyhow!("missing block number"))?;

    let current_time = block.timestamp.as_u32();

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

    // encode the call data
    let calldata = get_swap_exact_eth_for_tokens_calldata(&token, current_time, client).await?;

    let eth_to_send_with_tx = amount_of_token_to_purchase()?;

    // build the initial EIP-1559 transaction (no priority fee yet)
    let uniswap_swap_tx = Eip1559TransactionRequest {
        chain_id: Some(Chain::Mainnet.into()),
        max_priority_fee_per_gas: Some(U256::zero()), // initially zero, we’ll refine after simulation
        max_fee_per_gas: Some(adjusted_max_fee),
        gas: Some(U256::from(1_000_000u64)),
        to: Some(NameOrAddress::Address(uniswap_v2_router_address)),
        data: Some(calldata),
        nonce: Some(nonce),
        value: Some(eth_to_send_with_tx),
        ..Default::default()
    };

    // ============================================================
    // 2) FLASHBOTS MIDDLEWARE
    // ============================================================
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
        wallet,
    );

    // ============================================================
    // 3) BUILD A BUNDLE WITH ONLY ONE TX
    // ============================================================
    // sign the transaction
    let signature = client_signed
        .signer()
        .sign_transaction(&TypedTransaction::Eip1559(uniswap_swap_tx.clone()))
        .await?;

    let bundle = BundleRequest::new()
        .push_transaction(TypedTransaction::Eip1559(uniswap_swap_tx.clone()).rlp_signed(&signature))
        // simulate on next block
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    // ============================================================
    // 4) SIMULATE
    // ============================================================
    info!("Beginning bundle simulation.....");
    let simulated_bundle: SimulatedBundle = client_signed.inner().simulate_bundle(&bundle).await?;
    info!("Simulated bundle: {:?}", simulated_bundle);

    // check if simulation is success
    if !is_flashbot_simulation_success(&simulated_bundle) {
        error!("error simulating single-tx backrun");
        return Ok(());
    }
    info!("Single Flashbot transaction simulation succeeded.");

    // get gas used
    let gas_used = simulated_bundle
        .transactions
        .get(0)
        .ok_or_else(|| anyhow!("No transaction found in simulation result"))?
        .gas_used
        .low_u64();

    // compute transaction cost
    let transaction_cost =
        get_transaction_cost_in_eth(&uniswap_swap_tx, gas_used, adjusted_max_fee)?;

    // ============================================================
    // 5) DETERMINE MINER BRIBE AND SET AS PRIORITY FEE
    // ============================================================
    // in your requirement => "priority gas fee = 10% transaction cost"
    let miner_bribe_in_eth = transaction_cost * 0.10;
    debug!(
        "miner bribe => half of transaction cost => {} ETH",
        miner_bribe_in_eth
    );

    let bribe_u256 = f64_to_u256(miner_bribe_in_eth)?;
    // re-build the transaction => only difference is max_priority_fee_per_gas = bribe
    let backrun_tx_final = Eip1559TransactionRequest {
        max_priority_fee_per_gas: Some(bribe_u256),
        ..uniswap_swap_tx
    };

    // resign transaction
    let signature_final = client_signed
        .signer()
        .sign_transaction(&TypedTransaction::Eip1559(backrun_tx_final.clone()))
        .await?;

    // ============================================================
    // 6) SUBMIT FOR PRODUCTION
    // ============================================================
    let production_bundle = BundleRequest::new()
        .push_transaction(TypedTransaction::Eip1559(backrun_tx_final).rlp_signed(&signature_final))
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    let results = client_signed
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

// ========== HELPER FUNCTIONS ==========

fn is_flashbot_simulation_success(bundle: &SimulatedBundle) -> bool {
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
