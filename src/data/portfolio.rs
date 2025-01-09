use super::tokens::Erc20Token;
use crate::{
    app_config::TIME_ROUNDS,
    data::{
        token_data::{
            get_tokens, token_count_by_state, total_token_gas_cost, total_token_sales_revenue,
            total_token_spend,
        },
        tokens::{TokenLiquidity, TokenState},
    },
    utils::{
        tx::{amount_of_token_to_purchase, get_token_sell_interval},
        type_conversion::address_to_string,
    },
};
use ethers::utils::format_units;

impl Erc20Token {
    pub fn display_token_portfolio_time_interval(
        &self,
    ) -> anyhow::Result<([f64; TIME_ROUNDS], [f64; TIME_ROUNDS])> {
        let time_bought = get_token_sell_interval()?;
        let mut profit_per_interval = [0.0; TIME_ROUNDS];
        let mut roi_per_interval = [0.0; TIME_ROUNDS];

        let token_address = self.lowercase_address();

        for i in 1..TIME_ROUNDS {
            let profit = self.profit_at_time_interval_(i + 1)?;
            let roi = self.roi_at_time_interval(i + 1)?;

            profit_per_interval[i] = profit;
            roi_per_interval[i] = roi;
            if profit != 0.0 && roi != 0.0 {
                if i == 1 {
                    println!("Stats for {} ({})", self.name, token_address);
                }
                println!(
                    "{} secs => profit of {}, and roi of {} ({})",
                    time_bought * i as u32,
                    profit,
                    roi,
                    self.liquidity
                );
                println!("----------------------------------------------");
            }
        }

        Ok((profit_per_interval, roi_per_interval))
    }

    pub async fn display_token_portfolio(&self) -> anyhow::Result<(f64, f64)> {
        let token_address = self.lowercase_address();
        let profit = self.profit()?;
        let roi = self.roi()?;

        if profit != 0.0 {
            println!("----------------------------------------------");
            println!(
                "{} ({}) has profit of {}, and roi of {}",
                self.name, token_address, profit, roi
            );
            println!("----------------------------------------------");
        }

        Ok((profit, roi))
    }

    pub fn profit(&self) -> anyhow::Result<f64> {
        if self.state != TokenState::Sold {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let total_cost = eth_basis + self.tx_gas_cost;
        let profit = if self.eth_recieved_at_sale >= total_cost {
            let abs_profit = self.eth_recieved_at_sale - total_cost;
            abs_profit.as_u128() as i128
        } else {
            let abs_profit = total_cost - self.eth_recieved_at_sale;
            -(abs_profit.as_u128() as i128)
        };

        let profit = profit as f64 / 1e18_f64;

        Ok(profit)
    }

    pub fn roi(&self) -> anyhow::Result<f64> {
        if self.state != TokenState::Sold {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit()?;
        let roi = profit / eth_basis;

        Ok(roi)
    }

    pub fn profit_at_time_interval_(&self, interval: usize) -> anyhow::Result<f64> {
        if !self.is_sold_at_time[interval - 1] {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let total_cost = eth_basis + self.tx_gas_cost;
        let profit = if self.amount_sold_at_time[interval - 1] >= total_cost {
            let abs_profit = self.amount_sold_at_time[interval - 1] - total_cost;
            abs_profit.as_u128() as i128
        } else {
            let abs_profit = total_cost - self.amount_sold_at_time[interval - 1];
            -(abs_profit.as_u128() as i128)
        };

        let profit = profit as f64 / 1e18_f64;

        Ok(profit)
    }

    pub fn roi_at_time_interval(&self, interval: usize) -> anyhow::Result<f64> {
        if !self.is_sold_at_time[interval - 1] {
            return Ok(0_f64);
        }

        let eth_basis = amount_of_token_to_purchase()?;

        let eth_basis = eth_basis.as_u128() as f64 / 1e18_f64;

        let profit = self.profit_at_time_interval_(interval)?;
        let roi = profit / eth_basis;

        Ok(roi)
    }

    pub fn lowercase_address(&self) -> String {
        let address_string = address_to_string(self.address);

        return address_string.to_lowercase();
    }
}

pub async fn display_token_time_stats() -> anyhow::Result<()> {
    let tokens = get_tokens().await;
    let time_bought = get_token_sell_interval()?;

    let mut profit_micro_liquidity_per_interval = [0.0; TIME_ROUNDS];
    let mut profit_very_low_liquidity_per_interval = [0.0; TIME_ROUNDS];
    let mut profit_low_liquidity_per_interval = [0.0; TIME_ROUNDS];
    let mut profit_medium_liquidity_per_interval = [0.0; TIME_ROUNDS];
    let mut profit_high_liquidity_per_interval = [0.0; TIME_ROUNDS];

    let mut sum_profit_per_interval = [0.0; TIME_ROUNDS];
    let mut sum_roi_per_interval = [0.0; TIME_ROUNDS];
    let mut average_roi_per_interval = [0.0; TIME_ROUNDS];
    let mut tokens_sold_at_this_interval: [u32; TIME_ROUNDS] = [0; TIME_ROUNDS];
    println!("----------------------------------------------");
    println!("----------------TOKEN STATS------------------");
    println!("----------------------------------------------");
    for token in tokens.values() {
        let (profits, roi) = token.display_token_portfolio_time_interval()?;

        // Initialize or accumulate for profits
        if sum_profit_per_interval.is_empty() {
            // First token => just clone its entire vector
            sum_profit_per_interval = profits.clone();
        } else {
            // Add each element into the corresponding sum
            for (i, &p) in profits.iter().enumerate() {
                sum_profit_per_interval[i] += p;
                // if profit is exactly zero then token was not sold at this interval yet,
                // so do not count it when averaging out profit and roi
                tokens_sold_at_this_interval[i] += if p == 0.0 { 0 } else { 1 };

                match token.liquidity {
                    TokenLiquidity::Micro(_) => profit_micro_liquidity_per_interval[i] += p,
                    TokenLiquidity::VeryLow(_) => profit_very_low_liquidity_per_interval[i] += p,
                    TokenLiquidity::Low(_) => profit_low_liquidity_per_interval[i] += p,
                    TokenLiquidity::Medium(_) => profit_medium_liquidity_per_interval[i] += p,
                    TokenLiquidity::High(_) => profit_high_liquidity_per_interval[i] += p,
                    TokenLiquidity::Zero => {}
                }
            }
        }

        // Similarly for roi
        if sum_roi_per_interval.is_empty() {
            sum_roi_per_interval = roi.clone();
        } else {
            for (i, &r) in roi.iter().enumerate() {
                sum_roi_per_interval[i] += r;
            }
        }
    }

    for i in 0..TIME_ROUNDS {
        average_roi_per_interval[i] = if tokens_sold_at_this_interval[i] > 0 {
            sum_roi_per_interval[i] / tokens_sold_at_this_interval[i] as f64
        } else {
            0.0
        }
    }
    println!("----------------------------------------------");
    println!("------PROFIT PERFORMANCE BY TIME INTERVAL-----");
    println!("----------------------------------------------");

    for i in 1..TIME_ROUNDS {
        println!(
            "{} secs => profit of {}, and roi of {} ({} tokens sold)",
            time_bought * i as u32,
            sum_profit_per_interval[i],
            average_roi_per_interval[i],
            tokens_sold_at_this_interval[i]
        );
        if profit_micro_liquidity_per_interval[i] != 0.0 {
            println!(
                "micro liquidity => profit of {}",
                profit_micro_liquidity_per_interval[i]
            );
        }
        if profit_very_low_liquidity_per_interval[i] != 0.0 {
            println!(
                "very low liquidity => profit of {}",
                profit_very_low_liquidity_per_interval[i]
            );
        }
        if profit_low_liquidity_per_interval[i] != 0.0 {
            println!(
                "low liquidity => profit of {}",
                profit_low_liquidity_per_interval[i]
            );
        }
        if profit_medium_liquidity_per_interval[i] != 0.0 {
            println!(
                "medium liquidity => profit of {}",
                profit_medium_liquidity_per_interval[i]
            );
        }
        if profit_high_liquidity_per_interval[i] != 0.0 {
            println!(
                "high liquidity => profit of {}",
                profit_high_liquidity_per_interval[i]
            );
        }
        println!("----------------------------------------------");
    }
    // show addtional token data
    display_token_data().await?;

    Ok(())
}

pub async fn display_token_stats() -> anyhow::Result<()> {
    let tokens = get_tokens().await;

    let mut total_profit = 0.0;
    let mut sum_roi = 0.0;
    let mut tokens_sold: u32 = 0;
    println!("----------------------------------------------");
    println!("----------------TOKEN STATS------------------");
    println!("----------------------------------------------");
    for token in tokens.values() {
        let (profits, roi) = token.display_token_portfolio().await?;

        if profits != 0.0 {
            total_profit += profits;
            sum_roi += roi;
            tokens_sold += 1;
        }
    }

    let avg_roi = if tokens_sold > 0 {
        sum_roi / tokens_sold as f64
    } else {
        0.0
    };
    let avg_profit = if tokens_sold > 0 {
        total_profit / tokens_sold as f64
    } else {
        0.0
    };

    println!("----------------------------------------------");
    println!("------PROFIT PERFORMANCE ---------------------");
    println!("----------------------------------------------");

    println!("profit of {}, and roi of {}", total_profit, avg_roi);
    println!(
        "{} tokens sold, {} profit per token",
        tokens_sold, avg_profit
    );

    println!("----------------------------------------------");
    println!("----------------------------------------------");

    // show additional token data
    display_token_data().await?;

    Ok(())
}

pub async fn display_token_data() -> anyhow::Result<()> {
    let tokens_bought = token_count_by_state(TokenState::Bought).await;
    let tokens_sold = token_count_by_state(TokenState::Sold).await;
    let total_gas_spent = total_token_gas_cost().await;
    let total_token_cost = total_token_spend().await?;
    let total_eth_cost = total_token_cost + total_gas_spent;
    let _revenue = total_token_sales_revenue().await;

    let gas_cost = format_units(total_gas_spent, "ether")?;
    let eth_cost = format_units(total_eth_cost, "ether")?;

    println!("----------------------------------------------");
    println!("{} tokens bought", tokens_bought);
    println!("{} tokens sold", tokens_sold);
    println!("{} total eth spent", eth_cost);
    println!("{} total gas spent", gas_cost);
    println!("----------------------------------------------");

    Ok(())
}
