use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;

use crate::{
    app_config::{
        CHAIN, TOKEN_HOLDER_THRESHOLD_PERCENTAGE, TOKEN_LOCKERS_BASE, TOKEN_LOCKERS_MAINNET,
    },
    utils::type_conversion::{address_to_string, u256_to_f64},
    verify::{check_token_lock::TokenHolders, token_check::external_api::moralis},
};

use super::token_data::ERC20Token;

#[derive(Debug, Default)]
pub struct TokenHolderCheck {
    // pub creator_holder_percentage: f64,
    pub top_holder_percentage: f64,
    pub percentage_tokens_burned_or_locked: f64,
    // pub creator_owns_more_than_10_percent_of_tokens: bool,
    pub top_holder_more_than_10_percent_of_tokens: bool,
}

pub async fn get_token_holder_check(
    token: &ERC20Token,
    // creator_address: &str,
    client: &Arc<Provider<Ws>>,
) -> Result<Option<TokenHolderCheck>> {
    let total_supply = token.get_total_token_supply(client).await?;
    // Step 2) Retrieve top holder info. This is the part you'll have to implement
    //         with a subgraph or block explorer. For now, we assume a function:
    // fetch_top_lp_holders(pair_address) -> Vec<LpHolderInfo>
    let token_address = address_to_string(token.address);
    let top_holders: Vec<TokenHolders> = moralis::get_token_holder_list(&token_address).await?;

    if top_holders.is_empty() {
        // no token holders found yet!
        return Ok(None);
    }

    let mut top_holder = TokenHolders::default();
    // let mut creator_holdings = TokenHolders::default();

    // Step 3) Sum up balances for addresses in known_lockers
    let mut burnt_or_locked_balance = U256::zero();

    for info in top_holders.iter() {
        // // check creator holdings
        // if info.holder.to_lowercase() == creator_address.to_lowercase() {
        //     creator_holdings = TokenHolders {
        //         holder: info.holder.clone(),
        //         quantity: info.quantity,
        //     };
        // }

        let mut is_burned_or_locked = false;

        if CHAIN == Chain::Mainnet {
            // sum up all locked holdings
            if TOKEN_LOCKERS_MAINNET.contains(&info.holder.as_str()) {
                burnt_or_locked_balance += info.quantity;
                is_burned_or_locked = true;
            }
        } else {
            if TOKEN_LOCKERS_BASE.contains(&info.holder.as_str()) {
                burnt_or_locked_balance += info.quantity;
                is_burned_or_locked = true;
            }
        }

        // only check holders that are not burned or locked
        if !is_burned_or_locked {
            // find top holder
            if top_holder.quantity < info.quantity {
                top_holder = TokenHolders {
                    holder: info.holder.clone(),
                    quantity: info.quantity,
                };
            }
        }
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

    let token_holder_check = TokenHolderCheck {
        // creator_holder_percentage: 100_f64
        //     * u256_div_u256_to_f64(creator_holdings.quantity, total_supply),
        top_holder_percentage: 100_f64 * u256_div_u256_to_f64(top_holder.quantity, total_supply),
        // creator_owns_more_than_10_percent_of_tokens: creator_holdings.quantity
        //     > max_token_threshold,
        percentage_tokens_burned_or_locked: 100_f64
            * u256_div_u256_to_f64(burnt_or_locked_balance, total_supply),
        top_holder_more_than_10_percent_of_tokens: top_holder.quantity > max_token_threshold,
    };

    println!(
        "top holder more than 10% ? => {}",
        token_holder_check.top_holder_more_than_10_percent_of_tokens
    );

    Ok(Some(token_holder_check))
}

// CAREFULLY USING - make sure numerator < denominator to be safe
pub fn u256_div_u256_to_f64(numerator: U256, denominator: U256) -> f64 {
    let scale = U256::exp10(6);

    let scaled_value = numerator * scale / denominator;

    let value = u256_to_f64(scaled_value).unwrap() / 1e6_f64;

    value
}
