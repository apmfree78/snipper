use dotenv::dotenv;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Address, BlockNumber, U256};
use ethers::utils::format_units;
use snipper::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use snipper::abi::uniswap_pair::UNISWAP_PAIR;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_state_update::get_and_save_erc20_by_token_address;
use snipper::data::tokens::{Erc20Token, TokenState};
use snipper::events::PairCreatedEvent;
use snipper::swap::mainnet::setup::{TxType, TxWallet, WalletType};
use snipper::swap::tx_trait::Txs;
use snipper::token_tx::validate::check_all_tokens_are_tradable;
use std::str::FromStr;
use std::sync::Arc;
pub const PEPE: &str = "0x6982508145454Ce325dDbE47a25d4ec3d2311933";
pub const AIXBT: &str = "0x4f9fd6be4a90f2620860d680c0d4d5fb53d1a825";

struct TestSetup {
    tx_wallet: TxWallet,
    token: Erc20Token,
    last_block_timestamp: u32,
    sell_after: u32,
}

async fn setup(token_address: Address) -> anyhow::Result<TestSetup> {
    dotenv().ok();

    let ws_url = CONTRACT.get_address().ws_url.clone();
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

    let tx_wallet = TxWallet::new(WalletType::Main).await?;
    let tx_wallet_arc = Arc::new(tx_wallet.clone());

    // check token liquidity
    if let Err(error) = check_all_tokens_are_tradable(&tx_wallet_arc).await {
        println!("could not check token tradability => {}", error);
    }

    // for testing purposes assure it svalided
    token.set_state_to_(TokenState::Validated).await;

    Ok(TestSetup {
        tx_wallet,
        token,
        last_block_timestamp,
        sell_after,
    })
}

// SET CHAIN TO BASE TO TEST and SET TOKEN_TO_BUY_IN_ETH = 0.003
#[tokio::test]
async fn test_buy_sell_tx() -> anyhow::Result<()> {
    let aixbt_address = AIXBT.parse()?;
    let setup = setup(aixbt_address).await?;

    let eth = setup.tx_wallet.get_wallet_eth_balance().await?;

    let eth_balance = format_units(eth, "ether")?;
    println!("YOU HAVE {} of ETH at start", eth_balance);

    let tokens_bought = setup
        .tx_wallet
        .buy_tokens_for_eth(&setup.token, WalletType::Main, TxType::Real)
        .await?;

    let eth = setup.tx_wallet.get_wallet_eth_balance().await?;

    let eth_balance = format_units(eth, "ether")?;
    println!("YOU HAVE {} of ETH after buy", eth_balance);

    if tokens_bought > U256::zero() {
        let eth_recieved = setup
            .tx_wallet
            .sell_token_for_eth(&setup.token, WalletType::Main)
            .await?;
        let eth_readable = format_units(eth_recieved, "ether")?;
        println!("got {} eth from sale", eth_readable);
    }

    let eth = setup.tx_wallet.get_wallet_eth_balance().await?;

    let eth_balance = format_units(eth, "ether")?;
    println!("YOU HAVE {} of ETH after selling", eth_balance);
    Ok(())
}
