use anyhow::Result;
use dotenv::dotenv;
use ethers::{
    core::types::{Log, TxHash},
    providers::{Middleware, Provider, Ws},
    types::BlockNumber,
};
use futures::{lock::Mutex, stream, StreamExt};
use log::{error, info};
use snipper::{data::contracts::CHAIN, utils::logging::setup_logger};
use snipper::{data::contracts::CONTRACT, events};
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

    let provider = Provider::<Ws>::connect(ws_url).await?;
    let client = Arc::new(provider);
    info!("Connected to {:#?}", CHAIN);

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
            let last_timestamp = Arc::clone(&last_block_timestamp);

            match event {
                Ok(Event::Log(log)) => match events::decode_poolcreated_event(&log) {
                    Ok(pool_created_event) => {
                        info!("pool created event {:#?}", pool_created_event)
                    }
                    Err(error) => error!("error extracting pool created event => {}", error),
                },
                Ok(Event::Block(block)) => {
                    info!("NEW BLOCK ===> {}", block.timestamp);
                    let mut last_time = last_timestamp.lock().await;
                    let current_block_timestamp = block.timestamp.as_u32();

                    *last_time = current_block_timestamp;
                }
                Err(e) => error!("Error: {:?}", e),
            }
        })
        .await;

    Ok(())
}
