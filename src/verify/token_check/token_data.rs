use crate::abi::erc20::ERC20;
use crate::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use crate::abi::uniswap_pair::UNISWAP_PAIR;
use crate::data::contracts::CONTRACT;
use anyhow::Result;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, U256};
use log::{error, info, warn};
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub struct ERC20Token {
    // basic token data
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub address: Address,

    pub pair_address: Address, // uniswap pair address
    pub is_token_0: bool,      // if true token is token_0, otherwise its token_1
}

// token will get a score based on TokenCheckList
#[derive(Debug)]
pub enum TokenScore {
    Legit = 4,
    LikelyLegit = 3,
    Iffy = 2,
    LikelyScam = 1,
    Scam = 0,
}

/// get ERC20Token - struct that contains all data we need - from token address
pub async fn get_erc20_by_token_address(
    token_address: &str,
    client: &Arc<Provider<Ws>>,
) -> Result<Option<ERC20Token>> {
    info!("setup token contract...");
    let token_address_h160: Address = token_address.parse()?;
    let token_contract = ERC20::new(token_address_h160, client.clone());

    // get pair address of token, and is_token_0 , true if token is token_0, otherwise its token_1
    let (pair_address, is_token_0) =
        get_token_uniswap_v2_pair_address(token_address_h160, client).await?;

    // get basic toke data
    info!("getting basic token info...");
    let symbol = token_contract.symbol().call().await?;
    let decimals = token_contract.decimals().call().await?;
    let name = token_contract.name().call().await?;

    let token = ERC20Token {
        name,
        symbol,
        decimals,
        address: token_address_h160,
        pair_address,
        is_token_0,
        ..Default::default()
    };

    Ok(Some(token))
}

/// get the uniswap v2 pair address for the token and is_token_0, wether token is token_0 or
/// token_1
pub async fn get_token_uniswap_v2_pair_address(
    token_address: Address,
    client: &Arc<Provider<Ws>>,
) -> anyhow::Result<(Address, bool)> {
    // get required addresses from contracts.toml
    let uniswap_v2_factory_address: Address = CONTRACT.get_address().uniswap_factory.parse()?;
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;

    // determine pair addresses for RPC calls to eth node
    let uniswap_factory = UNISWAP_V2_FACTORY::new(uniswap_v2_factory_address, client.clone());
    let pair_address = uniswap_factory
        .get_pair(token_address, weth_address)
        .await?;
    let pair_contract = UNISWAP_PAIR::new(pair_address, client.clone());

    // get pair address of token
    let token_0 = pair_contract.token_0().call().await?;

    // check if token is token_0, otherwise its weth
    let is_token_0 = token_0 != weth_address;

    Ok((pair_address, is_token_0))
}
