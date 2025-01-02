// [16:30:48][snipper][INFO] pair created event PairCreatedEvent {
//     token0: 0xaa9c71781ca7ff63fd22f416e99b7b903089c9d0,
//     token1: 0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2,
//     pair: 0x153745f5e92d02add539ebdf75187b44a7859b28,
//     noname: 393214,
// }
// /[16:27:13][snipper][INFO] pair created event PairCreatedEvent {
//     token0: 0x38bca6b4c302a86d28281e56061699f735a32d45,
//     token1: 0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2,
//     pair: 0x6999ce929a6ce0b50df02de77ba80fd16391ed48,
//     noname: 393213,
// }/
//
use dotenv::dotenv;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Address, BlockNumber, U256};
use futures::lock::Mutex;
use snipper::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use snipper::abi::uniswap_pair::UNISWAP_PAIR;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_data::{
    check_all_tokens_are_tradable, display_token_time_stats, display_token_volume_stats,
    get_and_save_erc20_by_token_address, get_number_of_tokens, is_token_tradable,
};
use snipper::data::tokens::TokenState;
use snipper::events::PairCreatedEvent;
use snipper::swap::anvil::simlator::AnvilSimulator;
use snipper::swap::mainnet::setup::TxWallet;
use snipper::swap::tx_trait::Txs;
use snipper::token_tx::anvil::{buy_eligible_tokens_on_anvil, sell_eligible_tokens_on_anvil};
use snipper::token_tx::time_intervals::sell_eligible_tokens_at_time_intervals;
use snipper::token_tx::tx::buy_eligible_tokens;
use snipper::token_tx::volume_intervals::{
    mock_buy_eligible_tokens_at_volume_interval, mock_sell_eligible_tokens_at_volume_interval,
};
use std::str::FromStr;
use std::sync::Arc;

struct TestSetup {
    anvil_simulator: Arc<Mutex<AnvilSimulator>>,
    tx_wallet: Arc<TxWallet>,
    token_address: Address,
    last_block_timestamp: u32,
    sell_after: u32,
}

async fn setup(token_address: Address) -> anyhow::Result<TestSetup> {
    dotenv().ok();

    let ws_url = CONTRACT.get_address().ws_url.clone();
    let tx_wallet = TxWallet::new().await?;
    let tx_wallet = Arc::new(tx_wallet);

    let initial_block = tx_wallet
        .client
        .get_block(BlockNumber::Latest)
        .await?
        .unwrap();
    let last_block_timestamp = initial_block.timestamp.as_u32();
    println!("initial block timestamp => {}", last_block_timestamp);

    let sell_after_str =
        std::env::var("SELL_TOKEN_AFTER").expect("SELL_TOKEN_AFTER not found in .env");
    let sell_after = u32::from_str(&sell_after_str)?;

    let factory_address: Address = CONTRACT.get_address().uniswap_v2_factory.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    let factory = UNISWAP_V2_FACTORY::new(factory_address, tx_wallet.client());

    let pair_address = factory.get_pair(token_address, weth_address).call().await?;
    println!("pair address for WETH-TOKEN: {:?}", pair_address);

    let pair = UNISWAP_PAIR::new(pair_address, tx_wallet.client());
    let token_0 = pair.token_0().call().await?;
    let token_1 = pair.token_1().call().await?;

    let pair_created_event = PairCreatedEvent {
        token0: token_0,
        token1: token_1,
        pair: pair_address,
        noname: U256::from(0),
    };

    // Create an instance of AnvilSimulator
    let anvil_simulator = AnvilSimulator::new(&ws_url).await?;
    let anvil_simulator = Arc::new(Mutex::new(anvil_simulator));

    let token = get_and_save_erc20_by_token_address(&pair_created_event, &tx_wallet.client).await?;
    let token = token.unwrap();
    // check token liquidity
    if let Err(error) = check_all_tokens_are_tradable(&tx_wallet.client).await {
        println!("could not check token tradability => {}", error);
    }

    // for testing purposes assure it svalided
    token.set_state_to_(TokenState::Validated).await;

    Ok(TestSetup {
        tx_wallet,
        anvil_simulator,
        token_address,
        last_block_timestamp,
        sell_after,
    })
}

#[tokio::test]
#[ignore]
async fn test_anvil_meme_token_buy_sell_test() -> anyhow::Result<()> {
    // let token_address: Address = "0x616d4b42197cff456a80a8b93f6ebef2307dfb8c".parse()?;
    // let token_address: Address = "0xc5a07C9594C4d5138AA00feBbDEC048B6f0ad7D6".parse()?;
    // let token_address: Address = "0xaa9c71781ca7ff63fd22f416e99b7b903089c9d0".parse()?;
    let token_address: Address = "0x41dcd2bd1a261ff98d8e057154e0e7ce082a592f".parse()?;

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

    let anvil_lock = setup.anvil_simulator.lock().await;
    let mut token_balance = anvil_lock
        .get_wallet_token_balance_by_address(setup.token_address)
        .await?;
    assert!(token_balance > U256::from(0));

    setup.last_block_timestamp += setup.sell_after;

    if let Err(error) =
        sell_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running sell_eligible_tokens_on_anvil => {}", error);
    }

    number_of_tokens = get_number_of_tokens().await;

    let anvil_lock = setup.anvil_simulator.lock().await;
    token_balance = anvil_lock
        .get_wallet_token_balance_by_address(setup.token_address)
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

    let anvil_lock = setup.anvil_simulator.lock().await;
    let mut token_balance = anvil_lock
        .get_wallet_token_balance_by_address(setup.token_address)
        .await?;
    assert!(token_balance > U256::from(0));

    setup.last_block_timestamp += setup.sell_after;

    if let Err(error) =
        sell_eligible_tokens_on_anvil(&setup.anvil_simulator, setup.last_block_timestamp).await
    {
        println!("error running sell_eligible_tokens_on_anvil => {}", error);
    }

    number_of_tokens = get_number_of_tokens().await;

    let anvil_lock = setup.anvil_simulator.lock().await;
    token_balance = anvil_lock
        .get_wallet_token_balance_by_address(setup.token_address)
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

    let anvil_lock = setup.anvil_simulator.lock().await;
    let token_balance = anvil_lock
        .get_wallet_token_balance_by_address(setup.token_address)
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
    let anvil_lock = setup.anvil_simulator.lock().await;
    let new_token_balance = anvil_lock
        .get_wallet_token_balance_by_address(setup.token_address)
        .await?;
    assert_eq!(new_token_balance, token_balance);
    assert_eq!(number_of_tokens, 1);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_mock_token_buy_sell_test() -> anyhow::Result<()> {
    let token_address: Address = CONTRACT.get_address().link.parse()?;
    let mut setup = setup(token_address).await?;

    let mut number_of_tokens = get_number_of_tokens().await;
    assert_eq!(number_of_tokens, 1);

    let token_tradable = is_token_tradable(setup.token_address).await;
    assert!(token_tradable);

    if let Err(error) = mock_buy_eligible_tokens_at_volume_interval(
        &setup.tx_wallet.client,
        setup.last_block_timestamp,
    )
    .await
    {
        println!("error running buy_eligible_tokens_on_anvil => {}", error);
    }

    setup.last_block_timestamp += setup.sell_after;

    if let Err(error) = mock_sell_eligible_tokens_at_volume_interval(
        &setup.tx_wallet.client,
        setup.last_block_timestamp,
    )
    .await
    {
        println!("error running sell_eligible_tokens_on_anvil => {}", error);
    }

    if let Err(error) = display_token_volume_stats().await {
        println!("error displaying stats => {}", error);
    }

    number_of_tokens = get_number_of_tokens().await;

    assert_eq!(number_of_tokens, 1);

    Ok(())
}

#[tokio::test]
// #[ignore]
async fn test_mock_token_buy_sell_time_intervals_test() -> anyhow::Result<()> {
    let token_address: Address = CONTRACT.get_address().link.parse()?;
    let setup = setup(token_address).await?;
    let token_sell_interval =
        std::env::var("TOKEN_SELL_INTERVAL").expect("could not find TOKEN_SELL_INTERVAL in .env");
    let token_sell_interval: usize = token_sell_interval.parse()?;

    let mut number_of_tokens = get_number_of_tokens().await;
    assert_eq!(number_of_tokens, 1);

    let token_tradable = is_token_tradable(setup.token_address).await;
    assert!(token_tradable);

    if let Err(error) = buy_eligible_tokens(&setup.tx_wallet, setup.last_block_timestamp).await {
        println!("error running buy_eligible_tokens_on_anvil => {}", error);
    }

    for x in (token_sell_interval..=6000).step_by(token_sell_interval) {
        let sell_time = setup.last_block_timestamp + x as u32;
        if let Err(error) =
            sell_eligible_tokens_at_time_intervals(&setup.tx_wallet.client, sell_time).await
        {
            println!("error running sell_eligible_tokens_on_anvil => {}", error);
        }
    }

    if let Err(error) = display_token_time_stats().await {
        println!("error displaying stats => {}", error);
    }

    number_of_tokens = get_number_of_tokens().await;

    assert_eq!(number_of_tokens, 1);

    Ok(())
}
