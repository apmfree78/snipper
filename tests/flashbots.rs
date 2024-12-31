use dotenv::dotenv;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Address, BlockNumber, U256};
use snipper::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use snipper::abi::uniswap_pair::UNISWAP_PAIR;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_data::{
    check_all_tokens_are_tradable, get_and_save_erc20_by_token_address, set_token_to_,
};
use snipper::data::tokens::{Erc20Token, TokenState};
use snipper::events::PairCreatedEvent;
use snipper::swap::flashbots::flashbot_main::{
    prepare_and_submit_flashbot_token_purchase_tx, prepare_and_submit_flashbot_token_sell_tx,
};
use std::str::FromStr;
use std::sync::Arc;
pub const PEPE: &str = "0x6982508145454Ce325dDbE47a25d4ec3d2311933";

struct FlashbotTestSetup {
    client: Arc<Provider<Ws>>,
    token: Erc20Token,
    last_block_timestamp: u32,
    sell_after: u32,
}

async fn setup(token_address: Address) -> anyhow::Result<FlashbotTestSetup> {
    dotenv().ok();

    let ws_url = CONTRACT.get_address().alchemy_url.clone();
    println!("ws url => {}", ws_url);
    let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
    let client = Arc::new(provider);

    let initial_block = client.get_block(BlockNumber::Latest).await?.unwrap();
    let last_block_timestamp = initial_block.timestamp.as_u32();
    println!("initial block timestamp => {}", last_block_timestamp);

    let sell_after_str =
        std::env::var("SELL_TOKEN_AFTER").expect("SELL_TOKEN_AFTER not found in .env");
    let sell_after = u32::from_str(&sell_after_str)?;

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
        noname: U256::from(0),
    };

    let token = get_and_save_erc20_by_token_address(&pair_created_event, &client).await?;
    let token = token.unwrap();

    // check token liquidity
    if let Err(error) = check_all_tokens_are_tradable(&client).await {
        println!("could not check token tradability => {}", error);
    }

    // for testing purposes assure it svalided
    set_token_to_(TokenState::Validated, &token).await;

    Ok(FlashbotTestSetup {
        client,
        token,
        last_block_timestamp,
        sell_after,
    })
}

#[tokio::test]
#[ignore]
async fn test_flashbot_purchase_tx() -> anyhow::Result<()> {
    let token_address: Address = CONTRACT.get_address().link.parse()?;
    let setup = setup(token_address).await?;

    prepare_and_submit_flashbot_token_purchase_tx(&setup.token, &setup.client).await?;

    Ok(())
}

// SET CHAIN TO BASE TO TEST
#[tokio::test]
async fn test_flashbot_sell_tx() -> anyhow::Result<()> {
    let aixbt_address = PEPE.parse()?;
    let setup = setup(aixbt_address).await?;

    prepare_and_submit_flashbot_token_sell_tx(&setup.token, &setup.client).await?;

    Ok(())
}
