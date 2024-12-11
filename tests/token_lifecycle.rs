use dotenv::dotenv;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Address, BlockNumber};
use snipper::abi::uniswap_pool::UNISWAP_V3_POOL;
use snipper::abi::uniswap_v3_factory::UNISWAP_V3_FACTORY;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_data::{
    check_all_tokens_and_update_if_are_tradable, get_and_save_erc20_by_token_address,
};
use snipper::data::tokens::{buy_eligible_tokens_on_anvil, sell_eligible_tokens_on_anvil};
use snipper::events::PoolCreatedEvent;
use snipper::swap::anvil_simlator::AnvilSimulator;
use std::sync::Arc;

#[tokio::test]
async fn test_simulate_anvil_transaction_test() -> anyhow::Result<()> {
    dotenv().ok();

    let ws_url = CONTRACT.get_address().ws_url.clone();
    let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
    let client = Arc::new(provider);

    let initial_block = client.get_block(BlockNumber::Latest).await?.unwrap();
    let mut last_block_timestamp = initial_block.timestamp.as_u32();
    println!("initial block timestamp => {}", last_block_timestamp);

    let factory_address: Address = CONTRACT.get_address().uniswap_factory.parse()?;
    let link_address: Address = CONTRACT.get_address().link.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    let factory = UNISWAP_V3_FACTORY::new(factory_address, client.clone());

    let pool_address = factory
        .get_pool(link_address, weth_address, 10000u32)
        .call()
        .await?;
    println!("Pool address for WETH-LINK: {:?}", pool_address);

    let pool = UNISWAP_V3_POOL::new(pool_address, client.clone());
    let token_0 = pool.token_0().call().await?;
    let token_1 = pool.token_1().call().await?;
    let fee = pool.fee().call().await?;

    let pool_created_event = PoolCreatedEvent {
        token0: token_0,
        token1: token_1,
        fee,
        tick_spacing: 200,
        pool: pool_address,
    };

    get_and_save_erc20_by_token_address(&pool_created_event, &client).await?;

    // Create an instance of AnvilSimulator
    let anvil_simulator = AnvilSimulator::new(&ws_url).await?;
    let anvil_simulator = Arc::new(anvil_simulator);

    // check token liquidty
    if let Err(error) = check_all_tokens_and_update_if_are_tradable(&client).await {
        println!("could not check token tradability => {}", error);
    }

    if let Err(error) = buy_eligible_tokens_on_anvil(&anvil_simulator, last_block_timestamp).await {
        println!("error running buy_eligible_tokens_on_anvil => {}", error);
    }

    last_block_timestamp += 1800;

    if let Err(error) = sell_eligible_tokens_on_anvil(&anvil_simulator, last_block_timestamp).await
    {
        println!("error running sell_eligible_tokens_on_anvil => {}", error);
    }
    Ok(())
}
