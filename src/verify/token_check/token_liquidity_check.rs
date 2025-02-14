use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;

use crate::{
    app_config::{API_CHECK_LIMIT, CHAIN, TOKEN_LOCKERS_BASE, TOKEN_LOCKERS_MAINNET},
    utils::type_conversion::address_to_string,
    verify::{
        check_token_lock::TokenHolders,
        etherscan_api::get_token_holder_list,
        thegraph_api::fetch_uniswap_lp_holders,
        token_check::{external_api::moralis, token_holder_check::u256_div_u256_to_f64},
    },
};

use super::token_data::ERC20Token;

/// This function demonstrates how you might verify that >= 80% of the LP tokens
/// for a given Uniswap V2 pair are held by "locker" contracts, the burn address,
/// or any “safe” addresses you trust (like 0x00...dead).
///
/// # Arguments
/// * `pair_address` - The Uniswap V2 pair address (the LP token).
/// * `known_lockers` - A list of addresses known to be liquidity lock contracts
///                     (TeamFinance, Unicrypt, PinkLock, etc.) or the burn address.
/// * `provider` - An ethers provider to call the contract methods.
///
/// # Returns
/// Ok(Some(f64))  => threshold% of liquidity that is is locked
///
/// # Implementation Details
/// 1) We fetch the total supply of the pair (LP token).
/// 2) We retrieve top holders (the largest addresses that hold these LP tokens).
///    This step requires a subgraph or block explorer API you must implement.
/// 3) We sum up the balances for known lockers or burn addresses.
/// 4) divide locked/burned liquidity over total supply * 100.0
///
/// Final output struct you'd like to return
pub async fn get_percentage_liquidity_locked_or_burned(
    token: &ERC20Token,
    client: &Arc<Provider<Ws>>,
) -> Result<Option<f64>> {
    let total_supply = token.get_total_liquidity_token_supply(client).await?;
    // Step 2) Retrieve top holder info. This is the part you'll have to implement
    //         with a subgraph or etherscan block explorer api. For now, we assume a function:
    // fetch_top_lp_holders(pair_address) -> Vec<TokenHolders>
    let top_holders: Vec<TokenHolders> = if CHAIN == Chain::Base {
        let pair_address = address_to_string(token.pair_address);
        moralis::get_token_holder_list(&pair_address).await?
    } else {
        fetch_uniswap_lp_holders(token.pair_address).await?
    };

    if top_holders.is_empty() {
        // no token holders found yet!
        return Ok(None);
    }

    // Step 3) Sum up balances for addresses in known_lockers
    let mut locked_balance = U256::zero();

    // We'll treat addresses in `known_lockers` as well as any "dead" or "burn" addresses as locked
    // For example, 0x000000000000000000000000000000000000dEaD
    // or 0x0000000000000000000000000000000000000000 if you want
    // to do that, you can add them to the known_lockers array.
    let mut top_holder = TokenHolders::default();

    for info in top_holders.iter() {
        // find top holder
        if top_holder.quantity < info.quantity {
            top_holder = TokenHolders {
                holder: info.holder.clone(),
                quantity: info.quantity,
            };
        }

        if CHAIN == Chain::Mainnet {
            // sum up all locked holdings
            if TOKEN_LOCKERS_MAINNET.contains(&info.holder.as_str()) {
                locked_balance += info.quantity;
            }
        } else {
            if TOKEN_LOCKERS_BASE.contains(&info.holder.as_str()) {
                locked_balance += info.quantity;
            }
        }
    }

    // require
    if top_holder.quantity == U256::zero() {
        return Ok(None);
    }

    println!(
        "top holder for {} LP is {} with {}",
        token.name, top_holder.holder, top_holder.quantity
    );

    let percentage_of_liquidity_locked =
        100_f64 * u256_div_u256_to_f64(locked_balance, total_supply);

    Ok(Some(percentage_of_liquidity_locked))
}
