use dotenv::dotenv;
use ethers::providers::{Provider, Ws};
use ethers::types::Address;
use snipper::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use snipper::abi::uniswap_pair::UNISWAP_PAIR;
use snipper::app_config::AI_MODEL;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_state_update::get_and_save_erc20_by_token_address;
use snipper::data::tokens::Erc20Token;
use snipper::events::PairCreatedEvent;
use snipper::swap::mainnet::setup::{TxWallet, WalletType};
use snipper::verify::ai_submission::check_website_with_ai;
use snipper::{
    abi::erc20::ERC20,
    verify::etherscan_api::{get_source_code, get_token_info},
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

pub const WHITELIST_TOKENS: [&str; 4] = [
    "0x6982508145454Ce325dDbE47a25d4ec3d2311933",
    "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
    "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E",
    "0x514910771AF9Ca656af840dff83E8264EcF986CA",
];

async fn setup(token_address: Address, client: &Arc<Provider<Ws>>) -> anyhow::Result<Erc20Token> {
    dotenv().ok();

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

    let token = get_and_save_erc20_by_token_address(&pair_created_event, &client).await?;
    let token = token.unwrap();

    // let liquidity = token.get_liquidity(&client).await?;
    // if liquidity_is_not_zero_nor_micro(&liquidity) {
    //     token
    //         .set_to_tradable_plus_update_liquidity(&liquidity)
    //         .await;
    //     let liquidity_amount = extract_liquidity_amount(&liquidity).unwrap();
    //     println!(
    //         "{} has {} liquidity ({}) and ready for trading",
    //         liquidity_amount as f64 / 1e18_f64,
    //         token.name,
    //         liquidity
    //     );
    // }

    Ok(token)
}

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_audit_token_contract() -> anyhow::Result<()> {
    dotenv().ok();
    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";
    let source_code = get_source_code(VIRTUALS).await?;

    let audit = check_website_with_ai(source_code, &AI_MODEL)
        .await?
        .unwrap();
    println!("{:#?}", audit);

    // assert!(!source_code.is_empty());

    Ok(())
}

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_audit_token_contract_2() -> anyhow::Result<()> {
    dotenv().ok();
    const SCAM_TOKEN: &str = "0x1f035d740FD128E3818a08D613bC4C2D8f8Fccee";
    let source_code = get_source_code(SCAM_TOKEN).await?;

    let audit = check_website_with_ai(source_code, &AI_MODEL)
        .await?
        .unwrap();
    println!("AUDIT => {:#?}", audit);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_whitelist_contracts() -> anyhow::Result<()> {
    dotenv().ok();
    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

    for token in WHITELIST_TOKENS {
        let token_address: Address = token.parse()?;
        let contract = ERC20::new(token_address, tx_wallet.client.clone());
        let name = contract.name().call().await?;
        let source_code = get_source_code(token).await?;

        let audit = check_website_with_ai(source_code, &AI_MODEL)
            .await?
            .unwrap();
        println!("{} AUDIT => {:#?}", name, audit);
    }
    // assert!(!source_code.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_whitelist_get_info() -> anyhow::Result<()> {
    dotenv().ok();
    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

    for token in WHITELIST_TOKENS {
        let token_address: Address = token.parse()?;
        let contract = ERC20::new(token_address, tx_wallet.client.clone());
        let name = contract.name().call().await?;

        sleep(Duration::from_secs(1)).await;
        let token_info = get_token_info(token).await?;

        match token_info {
            Some(info) => {
                println!("INFO FOR {}, => {:#?}", name, info);
                assert!(!info.website.is_empty() || !info.twitter.is_empty());
            }
            None => println!("no token info avaliable!"),
        }
    }
    // assert!(!source_code.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_whitelist_ai_analysis() -> anyhow::Result<()> {
    dotenv().ok();
    let ws_url = CONTRACT.get_address().ws_url.clone();
    let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
    let client = Arc::new(provider);

    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

    for token in WHITELIST_TOKENS {
        let token_address: Address = token.parse()?;

        let token = setup(token_address, &client).await?;
        let contract = ERC20::new(token_address, tx_wallet.client.clone());
        let name = contract.name().call().await?;

        sleep(Duration::from_secs(7)).await;
        let ai_analysis = token.ai_analysis(&AI_MODEL).await?;

        match ai_analysis {
            Some(analysis) => {
                println!("analysis FOR {}, => {:#?}", name, analysis);
            }
            None => println!("no token analysis avaliable!"),
        }
    }
    // assert!(!source_code.is_empty());

    Ok(())
}
