use crate::data::token_data::get_tokens;
use crate::data::token_state_update::get_and_save_erc20_by_token_address;
use crate::data::token_state_update::remove_token;
use crate::data::tokens::extract_liquidity_amount;
use crate::data::tokens::Erc20Token;
use crate::data::tokens::TokenLiquidity;
use crate::data::tokens::TokenState;
use crate::events::PairCreatedEvent;
use crate::swap::anvil::validation::TokenLiquid;
use crate::swap::anvil::validation::TokenStatus;
use crate::swap::mainnet::setup::TxWallet;
use ethers::providers::{Provider, Ws};
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

            // check that if its a honeypot
            // *********************************
            let is_honepot = token.check_if_token_is_honeypot_and_update_state().await?;

            if !is_honepot {
                // *********************************
                // check that liqudity is locked
                let is_locked = token
                    .check_liquidity_is_locked_and_update_state(&tx_wallet.client)
                    .await?;

                if is_locked {
                    token.purchase(tx_wallet, current_time).await?;
                }
            }
            // *********************************
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

pub async fn check_all_tokens_are_tradable(client: &Arc<Provider<Ws>>) -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    for mut token in tokens.into_values() {
        // skip tokens in later phases
        if token.state == TokenState::Bought
            || token.state == TokenState::Sold
            || token.state == TokenState::Buying
            || token.state == TokenState::Selling
        {
            continue;
        }

        if !token.is_tradable {
            // check liquidity
            let liquidity = token.get_liquidity(client).await?;
            if liquidity_is_not_zero_nor_micro(&liquidity) {
                token
                    .set_to_tradable_plus_update_liquidity(&liquidity)
                    .await;
                let liquidity_amount = extract_liquidity_amount(&liquidity).unwrap();
                info!(
                    "{} has {} liquidity ({}) and ready for trading",
                    liquidity_amount as f64 / 1e18_f64,
                    token.name,
                    liquidity
                );

                // *********************************
                // check that if its a honeypot
                // *********************************
                let is_honepot = token.check_if_token_is_honeypot_and_update_state().await?;

                if !is_honepot {
                    // *********************************
                    // check that liqudity is locked
                    let _ = token
                        .check_liquidity_is_locked_and_update_state(client)
                        .await?;
                }

                // *********************************
            } else if liquidity != TokenLiquidity::Zero {
                let removed_token = remove_token(token.address).await.unwrap();
                warn!("micro liquidity scam token {} removed", removed_token.name);
            }
        } else if token.state != TokenState::Validated {
            println!("checking if honeypot data is avaliable for {}", token.name);
            let is_honepot = token.check_if_token_is_honeypot_and_update_state().await?;

            if !is_honepot {
                // *********************************
                // check that liqudity is locked
                let _ = token
                    .check_liquidity_is_locked_and_update_state(client)
                    .await?;
            }
        } else if token.state != TokenState::Locked {
            println!("checking if liquidity data is avaliable for {}", token.name);
            let _ = token
                .check_liquidity_is_locked_and_update_state(client)
                .await?;
        }
    }

    Ok(())
}

impl Erc20Token {
    pub async fn check_if_token_is_honeypot_and_update_state(&self) -> anyhow::Result<bool> {
        let is_honeypot = self.check_if_token_is_honeypot().await?;

        match is_honeypot {
            Some(honeypot) => {
                if !honeypot {
                    // do extra check with own simulated buy sell
                    let token_status = self
                        .validate_with_simulated_buy_sell(TokenLiquid::HasEnough)
                        .await?;
                    if token_status == TokenStatus::Legit {
                        self.set_state_to_(TokenState::Validated).await;
                        return Ok(false);
                    }
                }
                warn!("{} is a honeypot...removing!", self.name);
                remove_token(self.address).await;
                Ok(true)
            }
            None => {
                warn!(
                    "waiting for honeypot to provide token status for {}",
                    self.name
                );
                Ok(true)
            }
        }
    }
}

// pub async fn validate_tradable_tokens(client: &Arc<Provider<Ws>>) -> anyhow::Result<()> {
//     let tokens = get_tokens().await;
//
//     let mut handles = vec![];
//     for token_ref in tokens.values() {
//         let token = token_ref.clone();
//         let client = client.clone();
//
//         // SEPARATE THREAD FOR EACH TOKEN VALIDATION CHECK
//         let handle = tokio::spawn(async move {
//             let result: anyhow::Result<()> = async move {
//                 if token.is_tradable && token.state == TokenState::NotValidated {
//                     token.set_state_to_(TokenState::Validating).await;
//
//                     let token_status = token
//                         .validate_with_simulated_buy_sell(TokenLiquid::HasEnough)
//                         .await?;
//
//                     if token_status == TokenStatus::Legit {
//                         info!("{} is legit!", token.name);
//                         token.set_state_to_(TokenState::Validated).await;
//
//                         token
//                             .check_liquidity_is_locked_and_update_state(&client)
//                             .await?;
//                     } else {
//                         let scam_token = remove_token(token.address).await;
//                         let scam_token = scam_token.unwrap();
//                         warn!("removed {}", scam_token.symbol);
//                     }
//                 } else if token.state == TokenState::Validated {
//                     // check if liquidity is locked
//                     match is_liquidity_locked(&token, LIQUIDITY_PERCENTAGE_LOCKED, &client).await? {
//                         Some(is_locked) => {
//                             if is_locked {
//                                 token.set_state_to_(TokenState::Locked).await;
//                             } else {
//                                 println!(
//                                     "{} does not have locked liquidity... removing",
//                                     token.name
//                                 );
//                                 remove_token(token.address).await;
//                             }
//                         }
//                         None => {}
//                     }
//                 }
//                 Ok(())
//             }
//             .await;
//
//             if let Err(e) = result {
//                 error!("Error running validation thread: {:#}", e);
//             }
//         });
//
//         handles.push(handle);
//     }
//
//     Ok(())
// }
//
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
