use anyhow::Result;
use dotenv::dotenv;
use ethers::{
    core::types::{Log, TxHash},
    providers::{Middleware, Provider, Ws},
    types::BlockNumber,
};
use futures::{lock::Mutex, stream, StreamExt};
use log::{error, info, warn};
use snipper::{
    data::{
        contracts::CHAIN,
        token_data::check_all_tokens_and_update_if_are_tradable,
        tokens::{add_validate_buy_new_token, sell_eligible_tokens_on_anvil},
    },
    swap::anvil_simlator::AnvilSimulator,
    utils::logging::setup_logger,
};
use snipper::{
    data::{contracts::CONTRACT, tokens::buy_eligible_tokens_on_anvil},
    uniswap_v3_events,
};
use std::sync::Arc;

enum Event {
    Block(ethers::types::Block<TxHash>),
    Log(Log),
    // PendingTransactions(TxHash),
}

#[tokio::main]
async fn main() -> Result<()> {
    // initiate logger and environment variables
    dotenv().ok();
    setup_logger().expect("Failed to initialize logger.");
    let ws_url = CONTRACT.get_address().ws_url.clone();
    // setup provider

    let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
    let client = Arc::new(provider);
    info!("Connected to {:#?}", CHAIN);

    // creating anvil mainnet fork for testing
    info!("Connecting to Anvil...");
    let anvil = AnvilSimulator::new(&ws_url).await?;
    let anvil = Arc::new(anvil);
    info!("Anvil connected!");

    // TRACT TIME
    let initial_block = client.get_block(BlockNumber::Latest).await?.unwrap();
    let last_block_timestamp = initial_block.timestamp.as_u32();
    info!("initial block timestamp => {}", last_block_timestamp);
    let last_block_timestamp = Arc::new(Mutex::new(last_block_timestamp));

    let event_filter = uniswap_v3_events::set_signature_filter()?;
    // Create multiple subscription streams.
    let log_stream: stream::BoxStream<'_, Result<Event>> = client
        .subscribe_logs(&event_filter)
        .await?
        .map(|log| Ok(Event::Log(log)))
        .boxed();

    info!("Subscribed to aave v3 logs");

    // let tx_stream: stream::BoxStream<'_, Result<Event>> = client
    //     .subscribe_pending_txs()
    //     .await?
    //     .map(|tx| Ok(Event::PendingTransactions(tx)))
    //     .boxed();

    let block_stream: stream::BoxStream<'_, Result<Event>> = client
        .subscribe_blocks()
        .await?
        .map(|block| Ok(Event::Block(block)))
        .boxed();

    info!("Subscribed to pending transactions");

    // Merge the streams into a single stream.
    let combined_stream = stream::select_all(vec![log_stream, block_stream]);

    info!("Combined streams");

    combined_stream
        .for_each(|event| async {
            let client = Arc::clone(&client);
            let anvil = Arc::clone(&anvil);
            let last_timestamp = Arc::clone(&last_block_timestamp);

            match event {
                Ok(Event::Log(log)) => match uniswap_v3_events::decode_poolcreated_event(&log) {
                    Ok(pool_created_event) => {
                        info!("pool created event {:#?}", pool_created_event);
                        let last_time = last_timestamp.lock().await;

                        if let Err(error) = add_validate_buy_new_token(
                            &pool_created_event,
                            &client,
                            &anvil,
                            last_time.clone(),
                        )
                        .await
                        {
                            warn!("Could not run add_validate_buy_new_token => {}", error);
                        }
                    }
                    Err(error) => error!("error extracting pool created event => {}", error),
                },
                Ok(Event::Block(block)) => {
                    info!("NEW BLOCK ===> {}", block.timestamp);
                    let mut last_time = last_timestamp.lock().await;
                    let current_block_timestamp = block.timestamp.as_u32();

                    *last_time = current_block_timestamp;

                    // check token liquidty
                    if let Err(error) = check_all_tokens_and_update_if_are_tradable(&client).await {
                        error!("could not check token tradability => {}", error);
                    }

                    if let Err(error) =
                        buy_eligible_tokens_on_anvil(&anvil, current_block_timestamp).await
                    {
                        error!("error running buy_eligible_tokens_on_anvil => {}", error);
                    }

                    if let Err(error) =
                        sell_eligible_tokens_on_anvil(&anvil, current_block_timestamp).await
                    {
                        error!("error running sell_eligible_tokens_on_anvil => {}", error);
                    }
                }
                Err(e) => error!("Error: {:?}", e),
            }
        })
        .await;

    Ok(())
}
