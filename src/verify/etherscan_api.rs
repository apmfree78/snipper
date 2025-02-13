use anyhow::{anyhow, Result};
use ethers::types::{Address, U256};
use log::warn;
use reqwest::Client;
use serde::Deserialize;

use crate::{
    app_config::CHAIN,
    utils::type_conversion::{address_to_string, string_to_bool},
};

use super::check_token_lock::TokenHolders;

/// Internal structs mirroring Etherscan's JSON structure
#[derive(Debug, Deserialize)]
pub struct EtherscanResponse<T> {
    status: String,
    message: String,
    result: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct ContractOwner {
    #[serde(rename = "contractAddress")]
    _contract_address: String,

    #[serde(rename = "contractCreator")]
    contract_creator: String,

    #[serde(rename = "txHash")]
    _tx_hash: String,

    #[serde(rename = "blockNumber")]
    _block_number: String,

    #[serde(rename = "timestamp")]
    _timestamp: String,

    #[serde(rename = "contractFactory")]
    _contract_factory: String,

    #[serde(rename = "creationBytecode")]
    _creation_byte_code: String,
}

#[derive(Debug, Deserialize)]
struct ContractSourceCode {
    #[serde(rename = "SourceCode")]
    source_code: String,

    #[serde(rename = "ABI")]
    _abi: String,

    #[serde(rename = "ContractName")]
    _contract_name: String,

    #[serde(rename = "CompilerVersion")]
    _compiler_version: String,

    #[serde(rename = "OptimizationUsed")]
    _optimization_used: String,

    #[serde(rename = "Runs")]
    _runs: String,

    #[serde(rename = "ConstructorArguments")]
    _constructor_arguments: String,

    #[serde(rename = "EVMVersion")]
    _evm_version: String,

    #[serde(rename = "Library")]
    _library: String,

    #[serde(rename = "LicenseType")]
    _license_type: String,

    #[serde(rename = "Proxy")]
    _proxy: String,

    #[serde(rename = "Implementation")]
    _implementation: String,

    #[serde(rename = "SwarmSource")]
    _swarm_source: String,
}

/// Struct mirroring the fields returned by the "tokeninfo" action
#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    #[serde(rename = "contractAddress")]
    _contract_address: String,

    #[serde(rename = "tokenName")]
    _token_name: String,

    #[serde(rename = "symbol")]
    _symbol: String,

    #[serde(rename = "divisor")]
    _divisor: String,

    #[serde(rename = "tokenType")]
    _token_type: String,

    #[serde(rename = "totalSupply")]
    _total_supply: String,

    #[serde(rename = "blueCheckmark")]
    pub blue_checkmark: String,

    #[serde(rename = "description")]
    _description: String,

    #[serde(rename = "website")]
    pub website: String,

    #[serde(rename = "email")]
    _email: String,

    #[serde(rename = "blog")]
    _blog: String,

    #[serde(rename = "reddit")]
    _reddit: String,

    #[serde(rename = "slack")]
    _slack: String,

    #[serde(rename = "facebook")]
    _facebook: String,

    #[serde(rename = "twitter")]
    pub twitter: String,

    #[serde(rename = "bitcointalk")]
    _bitcointalk: String,

    #[serde(rename = "github")]
    _github: String,

    #[serde(rename = "telegram")]
    _telegram: String,

    #[serde(rename = "wechat")]
    _wechat: String,

    #[serde(rename = "linkedin")]
    _linkedin: String,

    #[serde(rename = "discord")]
    pub discord: String,

    #[serde(rename = "whitepaper")]
    pub whitepaper: String,

    #[serde(rename = "tokenPriceUSD")]
    _token_price_usd: String,

    #[serde(rename = "image")]
    _image: String,
}

#[derive(Debug, Deserialize)]
struct EtherscanHolderEntry {
    #[serde(rename = "TokenHolderAddress")]
    token_holder_address: String,

    #[serde(rename = "TokenHolderQuantity")]
    token_holder_quantity: String,
}

#[derive(Debug, Clone, Default)]
pub struct TokenWebData {
    pub blue_checkmark: bool,
    pub website: String,
    pub scraped_web_content: String,
    pub twitter: String,
    pub discord: String,
    pub whitepaper: String,
}

/// Example function to call Etherscan’s “tokenholderlist” endpoint.
///
/// # Arguments
///
/// - `contract_address`: The ERC-20 contract address (token address).
/// - `page`: The page number (starting from 1).
/// - `offset`: The number of holders per page (e.g. 10, 50, etc.).
/// - `api_key`: Your Etherscan API key.
///
/// # Returns
/// `Vec<TokenHolders>` with the holder address and quantity in `U256`.
///
// const chains = [42161, 8453, 10, 534352, 81457]
//
// for (const chain of chains) {
//
//   // endpoint accepts one chain at a time, loop for all your chains
//   const balance = fetch(`https://api.etherscan.io/v2/api?
//      chainid=${chain}
//      &module=account
//      &action=balance
//      &address=0xb5d85cbf7cb3ee0d56b3bb207d5fc4b82f43f511
//      &tag=latest&apikey=YourApiKeyToken`)
//
// }
pub async fn get_token_holder_list(contract_address: Address) -> Result<Vec<TokenHolders>> {
    // Build Etherscan URL
    // Example: https://api.basescan.org/api
    //   ?module=token
    //   &action=tokenholderlist
    //   &contractaddress=...
    //   &page=...
    //   &offset=...
    //   &apikey=...
    //
    let etherscan_api_key = get_etherscan_api_key()?;
    let contract_address_str = address_to_string(contract_address);

    let chain_id = CHAIN as u64;
    let etherscan_api = get_etherscan_api()?;

    let url = format!(
        "{}?chainid={}&module=token&action=tokenholderlist&contractaddress={}&apikey={}",
        etherscan_api, chain_id, contract_address_str, etherscan_api_key
    );

    // Make HTTP GET request
    let client = Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Request failed with status: {}", resp.status()));
    }

    // Parse JSON response
    let parsed: EtherscanResponse<EtherscanHolderEntry> = resp.json().await?;

    // Check Etherscan response status
    if parsed.status != "1" {
        return Err(anyhow!(
            "Etherscan returned status={}, message={}",
            parsed.status,
            parsed.message
        ));
    }

    // Convert to Vec<TokenHolders>
    let mut holders = Vec::with_capacity(parsed.result.len());
    for entry in parsed.result {
        let tokens_held = U256::from_dec_str(&entry.token_holder_quantity)?;

        holders.push(TokenHolders {
            holder: entry.token_holder_address,
            quantity: tokens_held,
        });
    }

    Ok(holders)
}

pub async fn get_source_code(contract_address: &str) -> Result<String> {
    // Build Etherscan URL
    // Example: https://api.basescan.org/api
    //   ?module=token
    //   &action=tokenholderlist
    //   &contractaddress=...
    //   &page=...
    //   &offset=...
    //   &apikey=...
    //
    let etherscan_api_key = get_etherscan_api_key()?;

    let chain_id = CHAIN as u64;
    let etherscan_api = get_etherscan_api()?;

    let url = format!(
        "{}?chainid={}&module=contract&action=getsourcecode&
address={}&apikey={}",
        etherscan_api, chain_id, contract_address, etherscan_api_key
    );

    // Make HTTP GET request
    let client = Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Request failed with status: {}", resp.status()));
    }

    // Parse JSON response
    let parsed: EtherscanResponse<ContractSourceCode> = resp.json().await?;

    // Check Etherscan response status
    if parsed.status != "1" {
        return Err(anyhow!(
            "Etherscan returned status={}, message={}",
            parsed.status,
            parsed.message
        ));
    }

    // Convert to Vec<TokenHolders>
    let source_code = match parsed.result.first() {
        Some(result) => result.source_code.clone(),
        None => String::new(),
    };

    Ok(source_code)
}

pub async fn get_contract_owner(contract_address: &str) -> Result<Option<String>> {
    let etherscan_api_key = get_etherscan_api_key()?;

    let chain_id = CHAIN as u64;
    let etherscan_api = get_etherscan_api()?;

    let url = format!(
        "{}?chainid={}&module=contract&action=getcontractcreation&
contractaddresses={}&apikey={}",
        etherscan_api, chain_id, contract_address, etherscan_api_key
    );

    // Make HTTP GET request
    let client = Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Request failed with status: {}", response.status()));
    }

    // Parse JSON response
    let parsed: EtherscanResponse<ContractOwner> = match response.json().await {
        Ok(parsed) => {
            // println!("parsed => {:#?}", parsed);
            parsed
        }
        Err(error) => {
            warn!("could not decode => {}", error);

            // // TESTING >>>>>>>>>>>>>>>>
            // sleep(Duration::from_millis(1000)).await;
            let response = client.get(&url).send().await?;
            let response_text = response.text().await?;
            println!("response_text => {}", response_text);

            return Ok(None);
        }
    };

    // Check Etherscan response status
    if parsed.status != "1" {
        return Err(anyhow!(
            "Etherscan returned status={}, message={}",
            parsed.status,
            parsed.message
        ));
    }

    // Convert to Vec<TokenHolders>
    match parsed.result.first() {
        Some(result) => Ok(Some(result.contract_creator.clone())),
        None => Ok(None),
    }
}

pub async fn get_token_info(contract_address: &str) -> Result<Option<TokenWebData>> {
    let etherscan_api_key = get_etherscan_api_key()?;

    let chain_id = CHAIN as u64;
    let etherscan_api = get_etherscan_api()?;

    let url = format!(
        "{}?chainid={}&module=token&action=tokeninfo&
contractaddress={}&apikey={}",
        etherscan_api, chain_id, contract_address, etherscan_api_key
    );

    // Make HTTP GET request
    let client = Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Request failed with status: {}", response.status()));
    }

    // Parse JSON response
    let parsed: EtherscanResponse<TokenInfo> = match response.json().await {
        Ok(parsed) => {
            // println!("parsed => {:#?}", parsed);
            parsed
        }
        Err(error) => {
            warn!("could not decode => {}", error);

            // // TESTING >>>>>>>>>>>>>>>>
            // sleep(Duration::from_millis(1000)).await;
            let response = client.get(&url).send().await?;
            let response_text = response.text().await?;
            println!("response_text => {}", response_text);

            return Ok(None);
        }
    };

    // Check Etherscan response status
    if parsed.status != "1" {
        return Err(anyhow!(
            "Etherscan returned status={}, message={}",
            parsed.status,
            parsed.message
        ));
    }

    // Convert to Vec<TokenHolders>
    match parsed.result.first() {
        Some(result) => Ok(Some(TokenWebData {
            website: result.website.clone(),
            blue_checkmark: string_to_bool(&result.blue_checkmark)?,
            twitter: result.twitter.clone(),
            discord: result.discord.clone(),
            whitepaper: result.whitepaper.clone(),
            ..Default::default()
        })),
        None => Ok(None),
    }
}

fn get_etherscan_api_key() -> anyhow::Result<String> {
    let etherscan_key =
        std::env::var("ETHERSCAN_API_KEY").expect("ETHERSCAN_API_KEY is not set in .env");

    Ok(etherscan_key)
}

fn get_etherscan_api() -> anyhow::Result<String> {
    let etherscan_key = std::env::var("ETHERSCAN_API").expect("ETHERSCAN_API is not set in .env");

    Ok(etherscan_key)
}
