use futures::lock::Mutex;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

use super::tokens::Erc20Token;

#[derive(Clone, Default, Debug)]
pub struct TokenStats {
    pub token: Erc20Token,
    pub profit: f32,
    pub time_of_sell: u32,
    pub roi: f32,
}

static PORTFOLIO: Lazy<Arc<Mutex<HashMap<String, TokenStats>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::<String, TokenStats>::new())));

pub async fn add_sold_token_to_mock_portfolio(
    token: &Erc20Token,
    timestamp: u32,
) -> anyhow::Result<()> {
    let portfolio = Arc::clone(&PORTFOLIO);
    let mut portfolio_lock = portfolio.lock().await;

    let profit = token.profit()?;
    let roi = token.roi()?;
    let token_address_string = token.lowercase_address();

    let token_stats = TokenStats {
        token: token.clone(),
        profit,
        time_of_sell: timestamp,
        roi,
    };

    portfolio_lock.insert(token_address_string, token_stats);

    Ok(())
}

pub async fn display_token_portfolio() -> anyhow::Result<()> {
    let portfolio = Arc::clone(&PORTFOLIO);
    let portfolio_lock = portfolio.lock().await;

    info!("----------------------------------------------");
    info!("----------------TOKEN STATS------------------");
    info!("----------------------------------------------");
    for token_stats in portfolio_lock.values() {
        let token_address = token_stats.token.lowercase_address();
        let profit = token_stats.token.profit()?;
        let roi = token_stats.token.roi()?;
        info!(
            "{} ({}) as profit of {}, and roi of {}",
            token_stats.token.name, token_address, profit, roi
        );
        info!("----------------------------------------------");
    }

    let total_profit: f32 = portfolio_lock
        .values()
        .map(|token_stats| token_stats.profit)
        .sum();
    info!("Total profit is ===> {}", total_profit);

    Ok(())
}
