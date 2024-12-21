use super::tokens::Erc20Token;
use crate::data::tokens::VOLUME_ROUNDS;
use crate::utils::type_conversion::format_to_5_decimals_decimal;
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
