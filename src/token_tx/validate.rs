use crate::data::token_data::get_and_save_erc20_by_token_address;
use crate::data::token_data::remove_token;
use crate::data::tokens::extract_liquidity_amount;
use crate::data::tokens::Erc20Token;
use crate::data::tokens::TokenLiquidity;
use crate::data::tokens::TokenState;
use crate::events::PairCreatedEvent;
use crate::swap::anvil::validation::TokenLiquid;
use crate::swap::anvil::validation::TokenStatus;
use crate::swap::mainnet::setup::TxWallet;
use log::info;
use log::warn;
use std::sync::Arc;

pub async fn add_validate_buy_new_token(
    pair_created_event: &PairCreatedEvent,
    tx_wallet: &Arc<TxWallet>,
    current_time: u32,
) -> anyhow::Result<()> {
    // SAVE TOKEN TO GLOBAL STATE
    if let Some(mut token) =
        get_and_save_erc20_by_token_address(&pair_created_event, &tx_wallet.client).await?
    {
        let liquidity = token.get_liquidity(&tx_wallet.client).await?;
        if liquidity_is_not_zero_nor_micro(&liquidity) {
            // TODO - set more conditons for tradibilty in production
            token
                .set_to_tradable_plus_update_liquidity(&liquidity)
                .await;
            let liquidity_amount = extract_liquidity_amount(&liquidity).unwrap();
            info!(
                "{} has {} ETH liquidity ({}) and ready for trading",
                token.name,
                liquidity_amount as f64 / 1e18_f64,
                liquidity
            );

            //******************************************
            // let _token_status = validate_token(&token).await?;

            // check that liqudity is locked
            let is_locked = token
                .validate_liquidity_is_locked(&tx_wallet.client)
                .await?;

            if is_locked {
                token.purchase(tx_wallet, current_time).await?;
            }
        } else {
            if liquidity == TokenLiquidity::Zero {
                info!("{} has no liquidity, cannot purchase yet!", token.name);
            } else {
                let removed_token = remove_token(token.address).await.unwrap();
                warn!("micro liquidity scam token {} removed", removed_token.name);
            }
        }
    }

    Ok(())
}

pub async fn validate_token(token: &Erc20Token) -> anyhow::Result<TokenStatus> {
    //******************************************
    token.set_state_to_(TokenState::Validating).await;
    let token_status = token
        .validate_with_simulated_buy_sell(TokenLiquid::HasEnough)
        .await?;

    if token_status == TokenStatus::Legit {
        info!("{} is legit!", token.name);
        token.set_state_to_(TokenState::Validated).await;
    } else {
        // cannot buy or sell token remove it
        let removed_token = remove_token(token.address).await.unwrap();
        warn!("scam token {} removed", removed_token.name);
    }
    // ********************************************
    Ok(token_status)
}

pub fn liquidity_is_not_zero_nor_micro(liquidity: &TokenLiquidity) -> bool {
    match liquidity {
        TokenLiquidity::Zero | TokenLiquidity::Micro(_) => false,
        _ => true,
    }
}

pub fn liquidity_is_high(liquidity: &TokenLiquidity) -> bool {
    match liquidity {
        TokenLiquidity::High(_) => true,
        _ => false,
    }
}

// pub async fn validate_token_from_mempool_and_buy(
//     token: &Erc20Token,
//     add_liquidity_tx: &Transaction,
//     tx_wallet: &Arc<TxWallet>,
//     current_time: u32,
// ) -> anyhow::Result<()> {
//     if token.state != TokenState::NotValidated {
//         return Ok(());
//     }
//
//     token.set_state_to_(TokenState::Validating).await;
//     let token_status = token
//         .validate_with_simulated_buy_sell(TokenLiquid::NeedToAdd(add_liquidity_tx.clone()))
//         .await?;
//
//     if token_status == TokenStatus::Legit {
//         info!("{} token validated from mempool!", token.name);
//         token.set_state_to_(TokenState::Validated).await;
//
//         // check if token is tradable
//         let has_enough_liquidity = token.has_enough_liquidity(&tx_wallet.client).await?;
//         if has_enough_liquidity {
//             token
//                 .set_to_tradable_plus_update_liquidity(&TokenLiquidity::VeryLow(10u128))
//                 .await;
//
//             // go ahead and purchase
//             token.purchase(tx_wallet, current_time).await?;
//         }
//     } else {
//         let scam_token = remove_token(token.address).await;
//         let scam_token = scam_token.unwrap();
//         warn!("removed (mempool) {}", scam_token.symbol);
//     }
//     Ok(())
// }
