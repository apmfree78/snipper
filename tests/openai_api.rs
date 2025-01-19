use dotenv::dotenv;
use ethers::types::Address;
use snipper::{
    abi::erc20::ERC20,
    swap::mainnet::setup::{TxWallet, WalletType},
    verify::{
        etherscan_api::{get_source_code, get_token_info},
        openai_api::audit_token_contract,
    },
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

pub const WHITELIST_TOKENS: [&str; 4] = [
    "0x6982508145454Ce325dDbE47a25d4ec3d2311933",
    "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
    "0x1151CB3d861920e07a38e03eEAd12C32178567F6",
    "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E",
];

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_audit_token_contract() -> anyhow::Result<()> {
    dotenv().ok();
    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";
    let source_code = get_source_code(VIRTUALS).await?;

    let audit = audit_token_contract(source_code).await?.unwrap();
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

    let audit = audit_token_contract(source_code).await?.unwrap();

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

        let audit = audit_token_contract(source_code).await?.unwrap();
        println!("{} AUDIT => {:#?}", name, audit);
    }
    // assert!(!source_code.is_empty());

    Ok(())
}

#[tokio::test]
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
