use anyhow::Result;
use dotenv::dotenv;
use ethers::{
    core::types::{Log, TxHash},
    providers::Middleware,
    types::{BlockNumber, Chain},
};
use futures::{lock::Mutex, stream, StreamExt};
use log::{error, info, warn};
use snipper::{
    app_config::{AppMode, APP_MODE, CHAIN},
    data::{nonce::intialize_nonce, token_data::display_token_stats},
    swap::mainnet::setup::TxWallet,
    token_tx::tx::sell_eligible_tokens,
};
use snipper::{
    data::token_data::display_token_time_stats,
    events,
    token_tx::{
        time_intervals::mock_sell_eligible_tokens_at_time_intervals, tx::buy_eligible_tokens,
        validate::add_validate_buy_new_token,
    },
};
use snipper::{
    data::token_data::{check_all_tokens_are_tradable, validate_tradable_tokens},
    mempool::detect_add_liquidity::detect_token_add_liquidity_and_validate,
    utils::logging::setup_logger,
};
use std::sync::Arc;

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

    // setup wallet for all rpc calls and txs
    let tx_wallet = TxWallet::new().await?;
    let tx_wallet = Arc::new(tx_wallet);
    info!("Connected to {:#?}", CHAIN);

    // setup global nonce
    intialize_nonce(&tx_wallet).await?;

    // TRACT TIME
    let initial_block = tx_wallet
        .client
        .get_block(BlockNumber::Latest)
        .await?
        .unwrap();
    let last_block_timestamp = initial_block.timestamp.as_u32();
    info!("initial block timestamp => {}", last_block_timestamp);
    let last_block_timestamp = Arc::new(Mutex::new(last_block_timestamp));

    let event_filter = events::set_signature_filter()?;
    // Create multiple subscription streams.
    let log_stream: stream::BoxStream<'_, Result<Event>> = tx_wallet
        .client
        .subscribe_logs(&event_filter)
        .await?
        .map(|log| Ok(Event::Log(log)))
        .boxed();

    info!("Subscribed to aave v3 logs");

    let block_stream: stream::BoxStream<'_, Result<Event>> = tx_wallet
        .client
        .subscribe_blocks()
        .await?
        .map(|block| Ok(Event::Block(block)))
        .boxed();

    info!("Subscribed to pending transactions");

    // Merge the streams into a single stream.
    let combined_stream = if CHAIN == Chain::Mainnet {
        let tx_stream: stream::BoxStream<'_, Result<Event>> = tx_wallet
            .client
            .subscribe_pending_txs()
            .await?
            .map(|tx| Ok(Event::PendingTransactions(tx)))
            .boxed();
        stream::select_all(vec![log_stream, block_stream, tx_stream])
    } else {
        // for L2s that do not support access to mempool pending txs
        stream::select_all(vec![log_stream, block_stream])
    };

    info!("Combined streams");

    combined_stream
        .for_each(|event| async {
            let last_timestamp = Arc::clone(&last_block_timestamp);
            let tx_wallet = Arc::clone(&tx_wallet);

            match event {
                Ok(Event::Log(log)) => match events::decode_pair_created_event(&log) {
                    Ok(pair_created_event) => {
                        // info!("pair created event {:#?}", pair_created_event);
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
                    // info!("NEW BLOCK ===> {}", block.timestamp);
                    let mut last_time = last_timestamp.lock().await;
                    let current_block_timestamp = block.timestamp.as_u32();

                    *last_time = current_block_timestamp;

                    // check token liquidty
                    if let Err(error) = check_all_tokens_are_tradable(&tx_wallet.client).await {
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

                    if APP_MODE == AppMode::Production {
                        if let Err(error) =
                            sell_eligible_tokens(&tx_wallet, current_block_timestamp).await
                        {
                            error!("error running sell_eligible_tokens => {}", error);
                        }
                    } else {
                        if let Err(error) = mock_sell_eligible_tokens_at_time_intervals(
                            &tx_wallet.client,
                            current_block_timestamp,
                        )
                        .await
                        {
                            error!(
                                "error running mock_sell_eligible_tokens_at_time_intervals => {}",
                                error
                            );
                        }
                    }

                    // display stats every 5 mins
                    match block.number {
                        Some(block_number) => {
                            if block_number.as_u64() % 30 == 0 {
                                if APP_MODE == AppMode::Production {
                                    if let Err(error) = display_token_stats().await {
                                        error!("error displaying stats => {}", error);
                                    }
                                } else {
                                    if let Err(error) = display_token_time_stats().await {
                                        error!("error displaying stats => {}", error);
                                    }
                                }
                            }
                        }
                        None => warn!("could not get block number!"),
                    }
                }
                Err(e) => error!("Error: {:?}", e),
            }
        })
        .await;

    Ok(())
}
