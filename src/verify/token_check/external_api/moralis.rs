use std::fmt::Debug;

use anyhow::anyhow;
use ethers::types::{Chain, U256};
use log::warn;
use reqwest::Error;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{
    app_config::CHAIN,
    verify::{check_token_lock::TokenHolders, etherscan_api::TokenWebData},
};

pub enum MoralisApiCallType {
    GetTokenHolders,
    GetTokenMetaData,
}

#[derive(Debug, Deserialize)]
struct MoralisResponse<T> {
    page: u32,
    page_size: u32,
    cursor: String,
    result: Vec<T>,
}

#[derive(Debug, Deserialize, Default)]
struct MoralisTokenHolder {
    owner_address: String,
    owner_address_label: Option<String>,
    entity: Option<String>,
    entity_logo: Option<String>,
    balance: String,
    balance_formatted: String,
    usd_value: Option<String>,
    is_contract: bool,
    percentage_relative_to_total_supply: f64,
}

#[derive(Debug, Deserialize)]
pub struct MoralisTokenMetadata {
    pub address: String,
    pub address_label: Option<String>,
    pub name: String,
    pub symbol: String,
    pub decimals: Option<String>,
    pub logo: Option<String>,
    pub logo_hash: Option<String>,
    pub thumbnail: Option<String>,
    pub total_supply: Option<String>,
    pub total_supply_formatted: Option<String>,
    pub fully_diluted_valuation: Option<String>,
    pub block_number: Option<String>,
    pub validated: Option<u32>,
    pub created_at: Option<String>,
    pub possible_spam: Option<bool>,
    pub verified_contract: Option<bool>,
    pub categories: Vec<String>,
    pub links: Links,
    pub security_score: Option<u32>,
    pub description: Option<String>,
    pub circulating_supply: Option<String>,
    pub market_cap: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Links {
    discord: Option<String>,
    reddit: Option<String>,
    telegram: Option<String>,
    twitter: Option<String>,
    website: Option<String>,
    moralis: Option<String>,
    medium: Option<String>,
    github: Option<String>,
    bitbucket: Option<String>,
    facebook: Option<String>,
    instagram: Option<String>,
    linkedin: Option<String>,
    tiktok: Option<String>,
    youtube: Option<String>,
}

/// get list of token holders with their corresponding balances (in U256)
pub async fn get_token_holder_list(token_address: &str) -> anyhow::Result<Vec<TokenHolders>> {
    let token_holders =
        moralis_api_call::<MoralisTokenHolder>(token_address, MoralisApiCallType::GetTokenHolders)
            .await?;
    // Convert to Vec<TokenHolders>
    let mut holders = Vec::with_capacity(token_holders.len());
    for entry in token_holders {
        let tokens_held = U256::from_dec_str(&entry.balance)?;

        holders.push(TokenHolders {
            holder: entry.owner_address,
            quantity: tokens_held,
        });
    }

    Ok(holders)
}

/// get list of token social media profiles, website, etc
pub async fn get_token_info(token_address: &str) -> anyhow::Result<Option<TokenWebData>> {
    let token_metadata = moralis_api_call::<MoralisTokenMetadata>(
        token_address,
        MoralisApiCallType::GetTokenMetaData,
    )
    .await?;

    match token_metadata.first() {
        Some(result) => Ok(Some(TokenWebData {
            website: result.links.website.clone().unwrap_or_default(),
            twitter: result.links.twitter.clone().unwrap_or_default(),
            discord: result.links.discord.clone().unwrap_or_default(),
            whitepaper: "".to_string(),
            ..Default::default()
        })),
        None => Ok(None),
    }
}

pub async fn moralis_api_call<T>(
    token_address: &str,
    api_call_type: MoralisApiCallType,
) -> anyhow::Result<Vec<T>>
where
    T: DeserializeOwned + Debug,
{
    // Replace with your actual API key
    let api_key = get_moralis_api_key()?;
    let root_url = get_moralis_api()?;
    let chain_id = get_moralis_chain_id();

    let url = match api_call_type {
        MoralisApiCallType::GetTokenMetaData => format!(
            "{}/erc20/metadata?chain={}&addresses={}&order=DESC",
            root_url, chain_id, token_address,
        ),
        MoralisApiCallType::GetTokenHolders => format!(
            "{}/erc20/{}/owners?chain={}&order=DESC",
            root_url, token_address, chain_id
        ),
    };

    println!("url => {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("accept", "application/json")
        .header("X-API-Key", api_key.clone())
        .send()
        .await?;

    // let response_text = response.text().await?;
    // println!("response => {}", response_text);
    if !response.status().is_success() {
        let response_text = response.text().await?;
        return Err(anyhow!("Request failed with: {}", response_text));
    }

    // Parse JSON response
    // Match on the api_call_type to parse the shape we expect:
    let vec_t: Vec<T> = match api_call_type {
        // Metadata endpoint -> directly a JSON array => parse as Vec<T>
        MoralisApiCallType::GetTokenMetaData => match response.json::<Vec<T>>().await {
            Ok(metadata) => metadata,
            Err(error) => {
                let response = client
                    .get(&url)
                    .header("accept", "application/json")
                    .header("X-API-Key", api_key.clone())
                    .send()
                    .await?;

                let response_text = response.text().await?;
                println!("response_text => {}", response_text);
                panic!("Failed parsing token metadata as Vec<T>: {}", error);
            }
        },

        // Holders endpoint -> JSON object with "result" => parse into MoralisResponse<T>, then get .result
        MoralisApiCallType::GetTokenHolders => {
            let moralis_response: MoralisResponse<T> = match response.json().await {
                Ok(token_holders) => token_holders,
                Err(_) => {
                    let response = client
                        .get(&url)
                        .header("accept", "application/json")
                        .header("X-API-Key", api_key.clone())
                        .send()
                        .await?;

                    let response_text = response.text().await?;
                    println!("response_text => {}", response_text);
                    return Ok(Vec::new());
                }
            };

            moralis_response.result
        }
    };

    Ok(vec_t)
}

pub fn get_moralis_chain_id() -> String {
    match CHAIN {
        Chain::Base => "base".to_string(),
        _ => "eth".to_string(),
    }
}

fn get_moralis_api_key() -> anyhow::Result<String> {
    let etherscan_key =
        std::env::var("MORALIS_API_KEY").expect("MORALIS_API_KEY is not set in .env");

    Ok(etherscan_key)
}

fn get_moralis_api() -> anyhow::Result<String> {
    let etherscan_key = std::env::var("MORALIS_API").expect("MORALIS_API is not set in .env");

    Ok(etherscan_key)
}
