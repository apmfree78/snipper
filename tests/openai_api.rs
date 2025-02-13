use dotenv::dotenv;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Address, U256};
use snipper::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use snipper::abi::uniswap_pair::UNISWAP_PAIR;
use snipper::data::contracts::CONTRACT;
use snipper::data::token_state_update::get_and_save_erc20_by_token_address;
use snipper::data::tokens::Erc20Token;
use snipper::events::PairCreatedEvent;
use snipper::swap::mainnet::setup::{TxWallet, WalletType};
use snipper::swap::tx_trait::Txs;
use snipper::verify::check_token_holders::get_token_holder_analysis;
use snipper::verify::token_check::external_api::moralis;
use snipper::{
    abi::erc20::ERC20,
    app_config::AI_MODEL,
    verify::{
        ai_submission::check_code_with_ai,
        etherscan_api::{get_contract_owner, get_source_code, get_token_info},
    },
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

pub const WHITELIST_TOKENS: [&str; 4] = [
    "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
    "0x6982508145454Ce325dDbE47a25d4ec3d2311933",
    "0x1151CB3d861920e07a38e03eEAd12C32178567F6",
    "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E",
];

pub const SCAMLIST_TOKENS: [&str; 4] = [
    "0xaff019720963fb45e13b745abfa10b946de8f4c9",
    "0x9a301ad1ae2ba1ecf8693a60de92e834f4429e8c",
    "0x7ea18f3dff39b4cede0d8b16fe05852e85024146",
    "0x8f806505a0677da5f9c4e8aff5bc9237b6cd154f",
];

struct SetupData {
    tx_wallet: Arc<TxWallet>,
    token: Erc20Token,
}

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_audit_token_contract() -> anyhow::Result<()> {
    dotenv().ok();
    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";
    let source_code = get_source_code(VIRTUALS).await?;

    let audit = check_code_with_ai(source_code, &AI_MODEL).await?.unwrap();
    println!("{:#?}", audit);

    // assert!(!source_code.is_empty());

    Ok(())
}

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_contract_creation() -> anyhow::Result<()> {
    dotenv().ok();
    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";
    let token_owner = get_contract_owner(VIRTUALS).await?;

    match token_owner {
        Some(data) => println!("{:#?}", data),
        None => println!("Opps..could not unwrap!"),
    }

    // assert!(!source_code.is_empty());

    Ok(())
}

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn get_holder_analysis() -> anyhow::Result<()> {
    dotenv().ok();

    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";
    let token_address: Address = VIRTUALS.parse()?;

    let info = setup(token_address).await?;

    let token_owner = get_contract_owner(VIRTUALS).await?;

    let owner = match token_owner {
        Some(data) => data,
        None => panic!("Opps..could not unwrap!"),
    };

    let token_holder_analysis =
        get_token_holder_analysis(&info.token, &owner, &info.tx_wallet.client)
            .await?
            .unwrap();

    println!("analysis => {:#?}", token_holder_analysis);

    Ok(())
}

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_audit_token_contract_2() -> anyhow::Result<()> {
    dotenv().ok();
    const SCAM_TOKEN: &str = "0x1f035d740FD128E3818a08D613bC4C2D8f8Fccee";
    let source_code = get_source_code(SCAM_TOKEN).await?;

    let audit = check_code_with_ai(source_code, &AI_MODEL).await?.unwrap();

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

        match check_code_with_ai(source_code, &AI_MODEL).await? {
            Some(audit) => println!("{} AUDIT => {:#?}", name, audit),
            None => println!("Opps..something went wrong!"),
        };
    }
    // assert!(!source_code.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_scamlist_contracts() -> anyhow::Result<()> {
    dotenv().ok();
    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

    for token in SCAMLIST_TOKENS {
        let token_address: Address = token.parse()?;
        let contract = ERC20::new(token_address, tx_wallet.client.clone());
        let name = contract.name().call().await?;
        let source_code = get_source_code(token).await?;

        match check_code_with_ai(source_code, &AI_MODEL).await? {
            Some(audit) => println!("{} AUDIT => {:#?}", name, audit),
            None => println!("Opps..something went wrong!"),
        };
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
async fn test_whitelist_get_info_using_moralis() -> anyhow::Result<()> {
    dotenv().ok();
    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

    for token in WHITELIST_TOKENS {
        let token_address: Address = token.parse()?;
        let contract = ERC20::new(token_address, tx_wallet.client.clone());
        let name = contract.name().call().await?;

        let token_info = moralis::get_token_info(token).await?;

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

async fn setup(token_address: Address) -> anyhow::Result<SetupData> {
    dotenv().ok();

    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

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

    let token = get_and_save_erc20_by_token_address(&pair_created_event, &tx_wallet.client).await?;
    let token = token.unwrap();

    let setup_data = SetupData { tx_wallet, token };

    Ok(setup_data)
}
