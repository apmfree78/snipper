use super::check_token_lock::TokenHolders;
use crate::utils::type_conversion::{address_to_string, f64_to_u256};
use anyhow::Result;
use ethers::types::Address;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

/// We'll define a GraphQL query for the first 1000 holders
const THEGRAPH_BASE_URL: &str = "https://gateway.thegraph.com/api";
const UNISWAP_V2_SUBGRAPH_ID: &str = "EYCKATKGBKLWvSfwvBjzfCBmGwYNdVkduYXVivCsLRFu";

#[derive(Debug, Deserialize)]
struct GraphQlResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct LiquidityPositionData {
    #[serde(rename = "liquidityPositions")]
    liquidity_positions: Vec<LiquidityPosition>,
}

#[derive(Debug, Deserialize)]
struct LiquidityPosition {
    user: LiquidityUser,
    #[serde(rename = "liquidityTokenBalance")]
    liquidity_token_balance: String, // Usually "BigDecimal" in the subgraph
}

#[derive(Debug, Deserialize)]
struct LiquidityUser {
    id: String, // user's address
}

/// This function queries the Uniswap V2 subgraph for liquidity positions
/// for a specific pair. It returns a Vec of (address, balanceAsString).
///
/// The `liquidityTokenBalance` here is in subgraph's decimal format. You might
/// need to parse it to your desired numeric type. Also note that subgraph
/// might not reflect real-time info if indexing is behind.
pub async fn fetch_uniswap_lp_holders(pair_address: Address) -> Result<Vec<TokenHolders>> {
    // GraphQL query
    let query = format!(
        r#"{{
            liquidityPositions(where: {{ pair: "{pair}" }}, first: 100) {{
                user {{ id }}
                liquidityTokenBalance
            }}
        }}"#,
        pair = address_to_string(pair_address).to_lowercase(),
    );

    let client = Client::new();
    let body = HashMap::from([("query", query)]);

    let thegraph_api_key = get_thegraph_api_key()?;
    let graphql_url = format!(
        "{}/{}/subgraphs/id/{}",
        THEGRAPH_BASE_URL, thegraph_api_key, UNISWAP_V2_SUBGRAPH_ID
    );

    let resp = client
        .post(graphql_url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?; // ensure we get an error if not 200

    let parsed: GraphQlResponse<LiquidityPositionData> = resp.json().await?;

    let mut result_vec = Vec::<TokenHolders>::new();
    for lp in parsed.data.liquidity_positions {
        let liquidity_balance = lp.liquidity_token_balance.parse::<f64>()?;
        let liquidity_balance_u256 = f64_to_u256(liquidity_balance)?;
        // user.id is the address in lowercase
        // liquidityTokenBalance is a string decimal
        result_vec.push(TokenHolders {
            holder: lp.user.id,
            quantity: liquidity_balance_u256,
        });
    }

    Ok(result_vec)
}

fn get_thegraph_api_key() -> anyhow::Result<String> {
    let thegraph_key =
        std::env::var("THEGRAPH_API_KEY").expect("THEGRAPH_API_KEY is not set in .env");

    Ok(thegraph_key)
}
