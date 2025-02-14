use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;

use crate::{
    app_config::{
        API_CHECK_LIMIT, CHAIN, TOKEN_HOLDER_THRESHOLD_PERCENTAGE, TOKEN_LOCKERS_BASE,
        TOKEN_LOCKERS_MAINNET,
    },
    data::tokens::Erc20Token,
    utils::type_conversion::{address_to_string, u256_to_f64, u256_to_f64_with_decimals},
    verify::{
        check_token_lock::TokenHolders, etherscan_api::get_token_holder_list,
        thegraph_api::fetch_uniswap_lp_holders, token_check::external_api::moralis,
    },
};

#[derive(Debug, Default)]
pub struct TokenHolderAnalysis {
    // pub creator_holder_percentage: f64,
    pub top_holder_percentage: f64,
    // pub creator_owns_more_than_10_percent_of_tokens: bool,
    pub top_holder_more_than_10_percent_of_tokens: bool,
}

pub async fn get_token_holder_analysis(
    token: &Erc20Token,
    // creator_address: &str,
    client: &Arc<Provider<Ws>>,
) -> Result<Option<TokenHolderAnalysis>> {
    // check api limit for this token is not reached
    // TODO - Create separate count for token holder check
    let api_count = token.graphql_check_count().await;
    if api_count > API_CHECK_LIMIT {
        println!("api limit reached for {}", token.name);
        return Ok(None);
    }

    let total_supply = token.get_total_token_supply(client).await?;

    let token_address = address_to_string(token.address);
    let top_holders: Vec<TokenHolders> = moralis::get_token_holder_list(&token_address).await?;

    //increment api count
    token.increment_graphql_checks().await;

    if top_holders.is_empty() {
        // no token holders found yet!
        return Ok(None);
    }

    let mut top_holder = TokenHolders::default();
    // let mut creator_holdings = TokenHolders::default();

    for info in top_holders.iter() {
        // find top holder
        if top_holder.quantity < info.quantity {
            top_holder = TokenHolders {
                holder: info.holder.clone(),
                quantity: info.quantity,
            };
        }

        // check creator holdings
        // if info.holder.to_lowercase() == creator_address.to_lowercase() {
        //     creator_holdings = TokenHolders {
        //         holder: info.holder.clone(),
        //         quantity: info.quantity,
        //     };
        // }
    }

    // require
    if top_holder.quantity == U256::zero() {
        return Ok(None);
    }

    println!(
        "top holder for {} token is {} with {}",
        token.name, top_holder.holder, top_holder.quantity
    );

    let max_token_threshold =
        total_supply * U256::from(TOKEN_HOLDER_THRESHOLD_PERCENTAGE as u64) / U256::from(100_u64);

    let token_holder_analysis = TokenHolderAnalysis {
        // creator_holder_percentage: u256_div_u256_to_f64(creator_holdings.quantity, total_supply),
        top_holder_percentage: 100_f64 * u256_div_u256_to_f64(top_holder.quantity, total_supply),
        // creator_owns_more_than_10_percent_of_tokens: creator_holdings.quantity
        //     > max_token_threshold,
        top_holder_more_than_10_percent_of_tokens: top_holder.quantity > max_token_threshold,
    };

    Ok(Some(token_holder_analysis))
}

// CAREFULLY USING - make sure numerator < denominator to be safe
fn u256_div_u256_to_f64(numerator: U256, denominator: U256) -> f64 {
    let scale = U256::exp10(18);

    let scaled_value = numerator * scale / denominator;

    let value = u256_to_f64(scaled_value).unwrap() / 1e18_f64;

    value
}
