use anyhow::Result;
use dotenv::dotenv;
use ethers::{
    core::types::{Log, TxHash},
    providers::{Middleware, Provider, Ws},
    types::BlockNumber,
};
use futures::{lock::Mutex, stream, StreamExt};
use log::{error, info, warn};
use snipper::swap::mainnet::setup::TxWallet;
use snipper::{
    data::{
        contracts::CHAIN,
        token_data::{check_all_tokens_are_tradable, validate_tradable_tokens},
    },
    mempool::detect_add_liquidity::detect_token_add_liquidity_and_validate,
    utils::logging::setup_logger,
};
use snipper::{
    data::{contracts::CONTRACT, token_data::display_token_time_stats},
    events,
    token_tx::{
        time_intervals::sell_eligible_tokens_at_time_intervals, tx::buy_eligible_tokens,
        validate::add_validate_buy_new_token,
    },
};
use std::sync::Arc;

pub enum AppMode {
    Production,
    Simulation,
}

pub const APP_MODE: AppMode = AppMode::Simulation;

enum Event {
    Block(ethers::types::Block<TxHash>),
    Log(Log),
    PendingTransactions(TxHash),
}

#[tokio::main]
async fn main() -> Result<()> {
    // initiate logger and environment variables
    dotenv().ok();
    setup_logger().expect("Failed to initialize logger.");
    // let ws_url = CONTRACT.get_address().ws_url.clone();
    let ws_url = CONTRACT.get_address().alchemy_url.clone();
    // setup provider

    let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
    let client = Arc::new(provider);
    let tx_wallet = TxWallet::new().await?;
    let tx_wallet = Arc::new(tx_wallet);
    info!("Connected to {:#?}", CHAIN);

    // TRACT TIME
    let initial_block = client.get_block(BlockNumber::Latest).await?.unwrap();
    let last_block_timestamp = initial_block.timestamp.as_u32();
    info!("initial block timestamp => {}", last_block_timestamp);
    let last_block_timestamp = Arc::new(Mutex::new(last_block_timestamp));

    let event_filter = events::set_signature_filter()?;
    // Create multiple subscription streams.
    let log_stream: stream::BoxStream<'_, Result<Event>> = client
        .subscribe_logs(&event_filter)
        .await?
        .map(|log| Ok(Event::Log(log)))
        .boxed();

    info!("Subscribed to aave v3 logs");

    let tx_stream: stream::BoxStream<'_, Result<Event>> = client
        .subscribe_pending_txs()
        .await?
        .map(|tx| Ok(Event::PendingTransactions(tx)))
        .boxed();

    let block_stream: stream::BoxStream<'_, Result<Event>> = client
        .subscribe_blocks()
        .await?
        .map(|block| Ok(Event::Block(block)))
        .boxed();

    info!("Subscribed to pending transactions");

    // Merge the streams into a single stream.
    let combined_stream = stream::select_all(vec![log_stream, block_stream, tx_stream]);

    info!("Combined streams");

    combined_stream
        .for_each(|event| async {
            let client = Arc::clone(&client);
            let last_timestamp = Arc::clone(&last_block_timestamp);
            let tx_wallet = Arc::clone(&tx_wallet);

            match event {
                Ok(Event::Log(log)) => match events::decode_pair_created_event(&log) {
                    Ok(pair_created_event) => {
                        info!("pair created event {:#?}", pair_created_event);
                        let current_time = {
                            let last_time = last_timestamp.lock().await;
                            last_time.clone()
                        };

                        if let Err(error) = add_validate_buy_new_token(
                            &pair_created_event,
                            &tx_wallet,
                            current_time,
                        )
                        .await
                        {
                            warn!("Could not run add_validate_buy_new_token => {}", error);
                        }
                    }
                    Err(error) => error!("error extracting pool created event => {}", error),
                },
                Ok(Event::PendingTransactions(tx)) => {
                    let current_time = {
                        let last_time = last_timestamp.lock().await;
                        last_time.clone()
                    };
                    if let Err(error) =
                        detect_token_add_liquidity_and_validate(tx, &tx_wallet, current_time).await
                    {
                        error!(
                            "problem with detect_token_add_liquidity_and_validate => {}",
                            error
                        );
                    }
                }
                Ok(Event::Block(block)) => {
                    info!("NEW BLOCK ===> {}", block.timestamp);
                    let mut last_time = last_timestamp.lock().await;
                    let current_block_timestamp = block.timestamp.as_u32();

                    *last_time = current_block_timestamp;

                    // check token liquidty
                    if let Err(error) = check_all_tokens_are_tradable(&client).await {
                        error!("could not check token tradability => {}", error);
                    }

                    // validate tokens
                    if let Err(error) = validate_tradable_tokens().await {
                        error!("could not validate tradable tokens => {}", error);
                    }

                    if let Err(error) =
                        buy_eligible_tokens(&tx_wallet, current_block_timestamp).await
                    {
                        error!("error running buy_eligible_tokens_on_anvil => {}", error);
                    }

                    if let Err(error) =
                        sell_eligible_tokens_at_time_intervals(&client, current_block_timestamp)
                            .await
                    {
                        error!("error running sell_eligible_tokens_on_anvil => {}", error);
                    }

                    // display stats every 5 mins
                    if current_block_timestamp % 300 == 0 {
                        if let Err(error) = display_token_time_stats().await {
                            error!("error displaying stats => {}", error);
                        }
                    }
                }
                Err(e) => error!("Error: {:?}", e),
            }
        })
        .await;

    Ok(())
}
