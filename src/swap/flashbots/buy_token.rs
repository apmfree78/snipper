use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use crate::utils::tx::{
    amount_of_token_to_purchase, calculate_next_block_base_fee,
    get_swap_exact_eth_for_tokens_calldata, get_transaction_cost_in_eth,
};
use anyhow::{anyhow, Result};
use ethers::core::k256::ecdsa::SigningKey;
use ethers::core::rand::thread_rng;
use ethers::signers::Wallet;
use ethers::types::{Address, Block, BlockId, BlockNumber, H256, U256, U64};
use ethers::utils::format_units;
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

type FlashbotsBroadcaster = BroadcasterMiddleware<Arc<Provider<Ws>>, LocalWallet>;

// TODO - set value() to submit ETH with transaction
pub async fn submit_single_flashbots_tx(
    token: &Erc20Token,
    client: &Arc<Provider<Ws>>,
) -> Result<()> {
    // ============================================================
    // 1) PREPARE
    // ============================================================

    // load private key
    // TODO - fn get_wallet
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY not found in .env file");
    let wallet = LocalWallet::from_str(&private_key)?.with_chain_id(Chain::Mainnet);

    let (block, block_number) = get_current_block(client).await?;

    // FOR swap exact eth ONLY
    // encode the call data
    let calldata =
        get_swap_exact_eth_for_tokens_calldata(&token, block.timestamp.as_u32(), client).await?;

    // FOR swap exact eth ONLY
    let eth_to_send_with_tx = amount_of_token_to_purchase()?;

    // refactor to return vec?
    let (uniswap_swap_tx, max_gas_fee) =
        prepare_uniswap_purchase_tx(calldata, eth_to_send_with_tx, &block, &wallet, client).await?;

    // ============================================================
    // 2) FLASHBOTS MIDDLEWARE
    // ============================================================

    let signed_client = generate_flashbot_signed_client_with_builders(&wallet, client)?;

    // ============================================================
    // 3) BUILD A BUNDLE WITH ONLY ONE TX
    // ============================================================
    // refactor to accept vec?
    let bundle =
        create_flashbot_bundle_with_tx(&uniswap_swap_tx, &signed_client, block_number).await?;

    // ============================================================
    // 4) SIMULATE
    // ============================================================
    let gas_used = simulate_flashbot_tx_and_get_gas_used(&bundle, &signed_client).await?;

    // compute transaction cost
    let transaction_cost = match gas_used {
        Some(gas) => get_transaction_cost_in_eth(&uniswap_swap_tx, gas, max_gas_fee)?,
        None => return Ok(()), // simulaton failed
    };

    // ============================================================
    // 5) DETERMINE MINER BRIBE AND SET AS PRIORITY FEE
    // ============================================================
    // in your requirement => "priority gas fee = 10% transaction cost"
    let miner_bribe = transaction_cost * U256::one() / U256::from(10);
    let miner_bribe_readable = format_units(miner_bribe, 18u32)?;
    debug!(
        "miner bribe => half of transaction cost => {} ETH",
        miner_bribe_readable
    );

    // ============================================================
    // 6) SUBMIT FOR PRODUCTION
    // ============================================================

    submit_production_flashbot_tx(&uniswap_swap_tx, block_number, miner_bribe, &signed_client)
        .await?;
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

// returns None of simulation fails
async fn simulate_flashbot_tx_and_get_gas_used(
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

async fn prepare_uniswap_purchase_tx(
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

async fn get_current_block(client: &Arc<Provider<Ws>>) -> anyhow::Result<(Block<H256>, U64)> {
    // get the latest block
    let block = client
        .get_block(BlockNumber::Latest)
        .await?
        .ok_or_else(|| {
            anyhow!("Could not retrieve the latest block for next_base_fee calculation")
        })?;

    // block number
    let block_number = block
        .number
        .ok_or_else(|| anyhow!("missing block number"))?;

    Ok((block, block_number))
}

async fn submit_production_flashbot_tx(
    tx: &Eip1559TransactionRequest,
    block_number: U64,
    miner_bribe: U256,
    signed_client: &SignerMiddleware<FlashbotsBroadcaster, Wallet<SigningKey>>,
) -> anyhow::Result<()> {
    // re-build the transaction => only difference is max_priority_fee_per_gas = bribe
    let backrun_tx_final = Eip1559TransactionRequest {
        max_priority_fee_per_gas: Some(miner_bribe),
        ..tx.clone()
    };

    // resign transaction
    let signature_final = signed_client
        .signer()
        .sign_transaction(&TypedTransaction::Eip1559(backrun_tx_final.clone()))
        .await?;

    let production_bundle = BundleRequest::new()
        .push_transaction(TypedTransaction::Eip1559(backrun_tx_final).rlp_signed(&signature_final))
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

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

fn generate_flashbot_signed_client_with_builders(
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

async fn create_flashbot_bundle_with_tx(
    tx_request: &Eip1559TransactionRequest,
    signed_client: &SignerMiddleware<FlashbotsBroadcaster, Wallet<SigningKey>>,
    block_number: U64,
) -> anyhow::Result<BundleRequest> {
    // sign the transaction
    let signature = signed_client
        .signer()
        .sign_transaction(&TypedTransaction::Eip1559(tx_request.clone()))
        .await?;

    let bundle = BundleRequest::new()
        .push_transaction(TypedTransaction::Eip1559(tx_request.clone()).rlp_signed(&signature))
        // simulate on next block
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    Ok(bundle)
}
