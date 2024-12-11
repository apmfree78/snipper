use dotenv::dotenv;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Address, BlockNumber, U256};
use snipper::abi::uniswap_pool::UNISWAP_V3_POOL;
use snipper::abi::uniswap_v3_factory::UNISWAP_V3_FACTORY;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_data::{
    check_all_tokens_and_update_if_are_tradable, get_and_save_erc20_by_token_address,
    get_number_of_tokens, is_token_tradable,
};
use snipper::data::tokens::{buy_eligible_tokens_on_anvil, sell_eligible_tokens_on_anvil};
use snipper::events::PoolCreatedEvent;
use snipper::swap::anvil_simlator::AnvilSimulator;
use std::str::FromStr;
use std::sync::Arc;

struct TestSetup {
    // client: Arc<Provider<Ws>>,
    anvil_simulator: Arc<AnvilSimulator>,
    token_address: Address,
    // weth_address: Address,
    last_block_timestamp: u32,
    sell_after: u32,
}

async fn setup(token_address: Address) -> anyhow::Result<TestSetup> {
    dotenv().ok();

    let ws_url = CONTRACT.get_address().ws_url.clone();
    let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
    let client = Arc::new(provider);

    let initial_block = client.get_block(BlockNumber::Latest).await?.unwrap();
    let last_block_timestamp = initial_block.timestamp.as_u32();
    println!("initial block timestamp => {}", last_block_timestamp);

    let sell_after_str =
        std::env::var("SELL_TOKEN_AFTER").expect("SELL_TOKEN_AFTER not found in .env");
    let sell_after = u32::from_str(&sell_after_str)?;

    let factory_address: Address = CONTRACT.get_address().uniswap_factory.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    let factory = UNISWAP_V3_FACTORY::new(factory_address, client.clone());

    let mut pool_address = factory
        .get_pool(token_address, weth_address, 10000u32)
        .call()
        .await?;
    println!("Pool address for WETH-TOKEN: {:?}", pool_address);

    if pool_address == Address::zero() {
        pool_address = factory
            .get_pool(weth_address, token_address, 10000u32)
            .call()
            .await?;
    }
    println!("Pool address for WETH-TOKEN: {:?}", pool_address);

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

    // check token liquidity
    if let Err(error) = check_all_tokens_and_update_if_are_tradable(&client).await {
        println!("could not check token tradability => {}", error);
    }

    Ok(TestSetup {
        anvil_simulator,
        token_address,
        last_block_timestamp,
        sell_after,
    })
}

#[tokio::test]
async fn test_anvil_ai_token_buy_sell_test() -> anyhow::Result<()> {
    let token_address: Address = "0x821b37dc08e534207d8beae9b42a60443fd067b2".parse()?;
    let mut setup = setup(token_address).await?;

    let mut number_of_tokens = get_number_of_tokens().await;
    assert_eq!(number_of_tokens, 1);

    let token_tradable = is_token_tradable(setup.token_address).await;
    assert!(token_tradable);

    if let Err(error) =
        buy_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running buy_eligible_tokens_on_anvil => {}", error);
    }

    let mut token_balance = setup
        .anvil_simulator
        .get_token_balance_by_address(setup.token_address)
        .await?;
    assert!(token_balance > U256::from(0));

    setup.last_block_timestamp += setup.sell_after;

    if let Err(error) =
        sell_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running sell_eligible_tokens_on_anvil => {}", error);
    }

    number_of_tokens = get_number_of_tokens().await;

    token_balance = setup
        .anvil_simulator
        .get_token_balance_by_address(setup.token_address)
        .await?;
    assert_eq!(token_balance, U256::from(0));
    assert_eq!(number_of_tokens, 0);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_anvil_token_buy_sell_test() -> anyhow::Result<()> {
    let token_address: Address = CONTRACT.get_address().link.parse()?;
    let mut setup = setup(token_address).await?;

    let mut number_of_tokens = get_number_of_tokens().await;
    assert_eq!(number_of_tokens, 1);

    let token_tradable = is_token_tradable(setup.token_address).await;
    assert!(token_tradable);

    if let Err(error) =
        buy_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running buy_eligible_tokens_on_anvil => {}", error);
    }

    let mut token_balance = setup
        .anvil_simulator
        .get_token_balance_by_address(setup.token_address)
        .await?;
    assert!(token_balance > U256::from(0));

    setup.last_block_timestamp += setup.sell_after;

    if let Err(error) =
        sell_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running sell_eligible_tokens_on_anvil => {}", error);
    }

    number_of_tokens = get_number_of_tokens().await;

    token_balance = setup
        .anvil_simulator
        .get_token_balance_by_address(setup.token_address)
        .await?;
    assert_eq!(token_balance, U256::from(0));
    assert_eq!(number_of_tokens, 0);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_anvil_token_buy_no_sell_test() -> anyhow::Result<()> {
    let token_address: Address = CONTRACT.get_address().link.parse()?;
    let mut setup = setup(token_address).await?;

    let mut number_of_tokens = get_number_of_tokens().await;
    assert_eq!(number_of_tokens, 1);

    let token_tradable = is_token_tradable(setup.token_address).await;
    assert!(token_tradable);

    if let Err(error) =
        buy_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running buy_eligible_tokens_on_anvil => {}", error);
    }

    let token_balance = setup
        .anvil_simulator
        .get_token_balance_by_address(setup.token_address)
        .await?;
    assert!(token_balance > U256::from(0));

    // Increase time by less than sell_after
    setup.last_block_timestamp += setup.sell_after - 10;

    if let Err(error) =
        sell_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running sell_eligible_tokens_on_anvil => {}", error);
    }

    number_of_tokens = get_number_of_tokens().await;
    let new_token_balance = setup
        .anvil_simulator
        .get_token_balance_by_address(setup.token_address)
        .await?;

    assert_eq!(new_token_balance, token_balance);
    assert_eq!(number_of_tokens, 1);

    Ok(())
}
