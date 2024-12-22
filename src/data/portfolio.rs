use super::tokens::Erc20Token;
use crate::{
    token_tx::{time_intervals::TIME_ROUNDS, volume_intervals::VOLUME_ROUNDS},
    utils::type_conversion::format_to_5_decimals_decimal,
};
use ethers::types::U256;
use log::info;

impl Erc20Token {
    pub fn display_token_portfolio_volume_interval(&self) -> anyhow::Result<()> {
        let amount_bought =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let amount_bought_ether = ethers::utils::parse_ether(amount_bought)?;

        let token_address = self.lowercase_address();

        println!("Stats for {} ({})", self.name, token_address);
        for i in 0..VOLUME_ROUNDS {
            let profit = self.profit_at_volume_interval_(i + 1)?;
            let roi = self.roi_at_volume_interval(i + 1)?;

            // let ether_used_to_buy = format_units(amount_bought_ether * U256::from(i + 1), 18u32)?;
            let ether_used_to_buy =
                format_to_5_decimals_decimal(amount_bought_ether * U256::from(i + 1), 18u32);
            println!(
                "{} ether => profit of {}, and roi of {}",
                ether_used_to_buy, profit, roi
            );
            println!("----------------------------------------------");
        }

        // let total_profit: f32 = portfolio_lock
        //     .values()
        //     .map(|token_stats| token_stats.profit)
        //     .sum();
        // info!("Total profit is ===> {}", total_profit);

        Ok(())
    }

    pub fn display_token_portfolio_time_interval(&self) -> anyhow::Result<()> {
        let time_bought =
            std::env::var("TOKEN_SELL_INTERVAL").expect("TOKEN_SELL_INTERVAL is not set in .env");
        let time_bought: u32 = time_bought.parse()?;

        let token_address = self.lowercase_address();

        println!("Stats for {} ({})", self.name, token_address);
        for i in 0..TIME_ROUNDS {
            let profit = self.profit_at_time_interval_(i + 1)?;
            let roi = self.roi_at_time_interval(i + 1)?;

            if profit != 0.0 && roi != 0.0 {
                println!(
                    "{} secs => profit of {}, and roi of {}",
                    time_bought * i as u32,
                    profit,
                    roi
                );
                println!("----------------------------------------------");
            }
        }

        // let total_profit: f32 = portfolio_lock
        //     .values()
        //     .map(|token_stats| token_stats.profit)
        //     .sum();
        // info!("Total profit is ===> {}", total_profit);

        Ok(())
    }

    pub async fn display_token_portfolio(&self) -> anyhow::Result<()> {
        println!("----------------------------------------------");
        println!("----------------TOKEN STATS------------------");
        println!("----------------------------------------------");
        let token_address = self.lowercase_address();
        let profit = self.profit()?;
        let roi = self.roi()?;
        info!(
            "{} ({}) has profit of {}, and roi of {}",
            self.name, token_address, profit, roi
        );
        info!("----------------------------------------------");

        Ok(())
    }
}
