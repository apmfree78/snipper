use super::tokens::Erc20Token;
use crate::{
    app_config::TIME_ROUNDS,
    token_tx::volume_intervals::VOLUME_ROUNDS,
    utils::{tx::get_token_sell_interval, type_conversion::format_to_5_decimals_decimal},
};
use ethers::types::U256;

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

        Ok(())
    }

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
}
