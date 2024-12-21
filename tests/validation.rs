use dotenv::dotenv;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Address, BlockNumber};
use snipper::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use snipper::abi::uniswap_pair::UNISWAP_PAIR;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_data::{
    check_all_tokens_are_tradable, get_and_save_erc20_by_token_address, is_token_tradable,
};
use snipper::data::tokens::Erc20Token;
use snipper::events::PairCreatedEvent;
use snipper::swap::anvil_simlator::AnvilSimulator;
use snipper::swap::anvil_validation::{validate_token_with_simulated_buy_sell, TokenStatus};
use std::sync::Arc;

struct TestSetupValidation {
    anvil_simulator: Arc<AnvilSimulator>,
    token: Erc20Token,
}

async fn setup(token_address: Address) -> anyhow::Result<TestSetupValidation> {
    dotenv().ok();

    let ws_url = CONTRACT.get_address().ws_url.clone();
    let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
    let client = Arc::new(provider);

    let initial_block = client.get_block(BlockNumber::Latest).await?.unwrap();
    let last_block_timestamp = initial_block.timestamp.as_u32();
    println!("initial block timestamp => {}", last_block_timestamp);

    let factory_address: Address = CONTRACT.get_address().uniswap_v2_factory.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    let factory = UNISWAP_V2_FACTORY::new(factory_address, client.clone());

    let pair_address = factory.get_pair(token_address, weth_address).call().await?;
    println!("pair address for WETH-TOKEN: {:?}", pair_address);

    let pair = UNISWAP_PAIR::new(pair_address, client.clone());
    let token_0 = pair.token_0().call().await?;
    let token_1 = pair.token_1().call().await?;

    let pair_created_event = PairCreatedEvent {
        token0: token_0,
        token1: token_1,
        pair: pair_address,
        noname: ethers::types::U256::from(0),
    };

    // Create an instance of AnvilSimulator
    let anvil_simulator = AnvilSimulator::new(&ws_url).await?;
    let anvil_simulator = Arc::new(anvil_simulator);

    let token = get_and_save_erc20_by_token_address(&pair_created_event, &client).await?;
    let token = token.unwrap();

    // check token liquidity
    if let Err(error) = check_all_tokens_are_tradable(&client).await {
        println!("could not check token tradability => {}", error);
    }

    Ok(TestSetupValidation {
        anvil_simulator,
        token,
    })
}

#[tokio::test]
async fn test_successful_token_validation() -> anyhow::Result<()> {
    let token_address: Address = "0xc5a07C9594C4d5138AA00feBbDEC048B6f0ad7D6".parse()?;
    // let token_address: Address = "0x9d265f238634e6436360c96bd8940e3d77fe3630".parse()?;

    let setup = setup(token_address).await?;

    let token_tradable = is_token_tradable(setup.token.address).await;
    assert!(token_tradable);

    let token_status = validate_token_with_simulated_buy_sell(&setup.token).await?;

    assert_eq!(token_status, TokenStatus::Legit);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_failed_token_validation() -> anyhow::Result<()> {
    let token_address: Address = "0x616d4b42197cff456a80a8b93f6ebef2307dfb8c".parse()?;
    let setup = setup(token_address).await?;

    let token_tradable = is_token_tradable(setup.token.address).await;
    assert!(token_tradable);

    let token_status = validate_token_with_simulated_buy_sell(&setup.token).await?;

    assert_eq!(token_status, TokenStatus::CannotSell);

    Ok(())
}
