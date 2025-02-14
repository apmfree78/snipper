use anyhow::Result;
use dotenv::dotenv;
use ethers::types::Address;
use log::info;
use snipper::abi::erc20::ERC20;
use snipper::swap::mainnet::setup::{TxWallet, WalletType};
use snipper::verify::token_check::token_checklist::generate_token_checklist;
use snipper::verify::token_check::token_data::{get_token_uniswap_v2_pair_address, ERC20Token};
use std::sync::Arc;

// mainnet
pub const WHITELIST_TOKENS: [&str; 4] = [
    "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
    "0x6982508145454Ce325dDbE47a25d4ec3d2311933",
    "0x1151CB3d861920e07a38e03eEAd12C32178567F6",
    "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E",
];

// base ?
pub const SCAMLIST_TOKENS: [&str; 4] = [
    "0xaff019720963fb45e13b745abfa10b946de8f4c9",
    "0x9a301ad1ae2ba1ecf8693a60de92e834f4429e8c",
    "0x7ea18f3dff39b4cede0d8b16fe05852e85024146",
    "0x8f806505a0677da5f9c4e8aff5bc9237b6cd154f",
];

pub struct SetupData {
    pub tx_wallet: Arc<TxWallet>,
    pub token: ERC20Token,
}

#[tokio::test]
async fn test_generate_checklist() -> anyhow::Result<()> {
    const SCAM: &str = "0x9a301ad1ae2ba1ecf8693a60de92e834f4429e8c";
    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";
    let data = setup(VIRTUALS).await?;

    let token_checklist = generate_token_checklist(data.token, &data.tx_wallet.client).await?;

    println!("token checklist => {:#?}", token_checklist);

    Ok(())
}

/// get ERC20Token - struct that contains all data we need - from token address
pub async fn setup(token_address: &str) -> Result<SetupData> {
    dotenv().ok();
    let tx_wallet = TxWallet::new(WalletType::Test).await?;
    let tx_wallet = Arc::new(tx_wallet);

    let token_address_h160: Address = token_address.parse()?;
    let token_contract = ERC20::new(token_address_h160, tx_wallet.client.clone());

    // get basic toke data
    let symbol = token_contract.symbol().call().await?;
    let decimals = token_contract.decimals().call().await?;
    let name = token_contract.name().call().await?;

    // get pair address of token, and is_token_0 , true if token is token_0, otherwise its token_1
    println!("get pair address..");
    let (pair_address, is_token_0) =
        get_token_uniswap_v2_pair_address(token_address_h160, &tx_wallet.client).await?;

    let token = ERC20Token {
        name,
        symbol,
        decimals,
        address: token_address_h160,
        pair_address,
        is_token_0,
        ..Default::default()
    };

    Ok(SetupData { tx_wallet, token })
}
