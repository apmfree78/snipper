use dotenv::dotenv;
use ethers::types::Address;
use snipper::{
    abi::erc20::ERC20,
    data::token_state_update::get_openai_token_count,
    swap::mainnet::setup::{TxWallet, WalletType},
    utils::web_scrapper::scrape_site_and_get_text,
    verify::{
        etherscan_api::{get_source_code, get_token_info},
        openai_api::audit_token_contract,
    },
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

pub const WHITELIST_TOKENS: [&str; 3] = [
    "0x6982508145454Ce325dDbE47a25d4ec3d2311933",
    "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
    // "0x1151CB3d861920e07a38e03eEAd12C32178567F6",
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

#[tokio::test]
async fn test_scrapper() -> anyhow::Result<()> {
    dotenv().ok();
    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

    for token in WHITELIST_TOKENS {
        let token_address: Address = token.parse()?;
        let contract = ERC20::new(token_address, tx_wallet.client.clone());
        let name = contract.name().call().await?;

        sleep(Duration::from_secs(1)).await;
        let token_info = get_token_info(token).await?.unwrap();
        println!("website content for {} ....", name);
        let website_text = scrape_site_and_get_text(&token_info.website).await?;
        let tokens = get_openai_token_count(&website_text);
        println!("token count => {}", tokens);
        assert!(!website_text.is_empty());
    }

    Ok(())
}
