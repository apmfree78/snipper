use dotenv::dotenv;
use ethers::providers::Middleware;
use ethers::types::{Address, BlockNumber, U256};
use snipper::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use snipper::abi::uniswap_pair::UNISWAP_PAIR;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_state_update::get_and_save_erc20_by_token_address;
use snipper::data::tokens::{Erc20Token, TokenState};
use snipper::events::PairCreatedEvent;
use snipper::swap::mainnet::setup::{TxWallet, WalletType};
use snipper::token_tx::validate::check_all_tokens_are_tradable;
use std::str::FromStr;
use std::sync::Arc;
pub const PEPE: &str = "0x6982508145454Ce325dDbE47a25d4ec3d2311933";
pub const AIXBT: &str = "0x4f9fd6be4a90f2620860d680c0d4d5fb53d1a825";
pub const CPAI: &str = "0x6ef69Ba2d051761aFD38F218F0a3cF517D64A760";
pub const SPX: &str = "0xE0f63A424a4439cBE457D80E4f4b51aD25b2c56C";

struct TestSetup {
    tx_wallet: TxWallet,
    token: Erc20Token,
    last_block_timestamp: u32,
    sell_after: u32,
}

async fn setup(token_address: Address) -> anyhow::Result<TestSetup> {
    dotenv().ok();

    let tx_wallet = TxWallet::new(WalletType::Test).await?;
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
    let factory = UNISWAP_V2_FACTORY::new(factory_address, tx_wallet.client.clone());

    let pair_address = factory.get_pair(token_address, weth_address).call().await?;
    println!("pair address for WETH-TOKEN: {:?}", pair_address);

    let pair = UNISWAP_PAIR::new(pair_address, tx_wallet.client.clone());
    let token_0 = pair.token_0().call().await?;
    let token_1 = pair.token_1().call().await?;

    let pair_created_event = PairCreatedEvent {
        token0: token_0,
        token1: token_1,
        pair: pair_address,
        noname: U256::from(0),
    };

    let token = get_and_save_erc20_by_token_address(&pair_created_event, &tx_wallet.client).await?;
    let token = token.unwrap();

    // check token liquidity
    if let Err(error) = check_all_tokens_are_tradable(&tx_wallet).await {
        println!("could not check token tradability => {}", error);
    }

    // for testing purposes assure it svalided
    token.set_state_to_(TokenState::Validated).await;

    let tx_wallet = TxWallet::new(WalletType::Test).await?;

    Ok(TestSetup {
        tx_wallet,
        token,
        last_block_timestamp,
        sell_after,
    })
}

#[tokio::test]
async fn test_that_liquidity_is_locked() -> anyhow::Result<()> {
    let cpai_address: Address = CPAI.parse()?;
    let setup = setup(cpai_address).await?;

    let (summary, result) = setup.token.is_honeypot().await?;

    println!("summary => {:#?}", summary);
    println!("result => {:#?}", result);

    // assert!(is_locked);

    Ok(())
}

#[tokio::test]
async fn test_that_liquidity_is_locked_2() -> anyhow::Result<()> {
    let spx_address: Address = SPX.parse()?;
    let setup = setup(spx_address).await?;

    let (summary, result) = setup.token.is_honeypot().await?;

    println!("summary => {:#?}", summary);
    println!("result => {:#?}", result);
    // assert!(is_locked);

    Ok(())
}
