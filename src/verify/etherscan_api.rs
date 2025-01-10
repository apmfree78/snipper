use anyhow::{anyhow, Result};
use ethers::types::{Address, U256};
use reqwest::Client;
use serde::Deserialize;

use crate::{app_config::CHAIN, utils::type_conversion::address_to_string};

use super::check_token_lock::TokenHolders;

/// Internal structs mirroring Etherscan's JSON structure
#[derive(Debug, Deserialize)]
struct EtherscanResponse {
    status: String,
    message: String,
    result: Vec<EtherscanHolderEntry>,
}

#[derive(Debug, Deserialize)]
struct EtherscanHolderEntry {
    #[serde(rename = "TokenHolderAddress")]
    token_holder_address: String,

    #[serde(rename = "TokenHolderQuantity")]
    token_holder_quantity: String,
}

/// Example function to call Etherscan’s “tokenholderlist” endpoint.
///
/// # Arguments
///
/// - `contract_address`: The ERC-20 contract address (LP token address).
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
/// # Example
/// ```ignore
/// let holders = get_token_holder_list(
///     "0xaaaebe6fe48e54f431b0c390cfaf0b017d09d42d",
///     1,
///     10,
///     "YourApiKeyToken"
/// ).await?;
/// println!("{:?}", holders);
/// ```
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

    let url = format!(
        "https://api.etherscan.io/v2/api?chainid={}&module=token&action=tokenholderlist&contractaddress={}&apikey={}",
        chain_id,contract_address_str, etherscan_api_key
    );

    // Make HTTP GET request
    let client = Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Request failed with status: {}", resp.status()));
    }

    // Parse JSON response
    let parsed: EtherscanResponse = resp.json().await?;

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

fn get_etherscan_api_key() -> anyhow::Result<String> {
    let etherscan_key =
        std::env::var("ETHERSCAN_API_KEY").expect("ETHERSCAN_API_KEY is not set in .env");

    Ok(etherscan_key)
}
