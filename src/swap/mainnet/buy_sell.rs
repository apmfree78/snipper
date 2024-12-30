use crate::abi::erc20::ERC20;
use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::data::contracts::CONTRACT;
use crate::data::gas::update_tx_gas_cost_data;
use crate::data::tokens::Erc20Token;
use crate::utils::tx::{amount_of_token_to_purchase, get_amount_out_uniswap_v2, TxSlippage};
use crate::utils::type_conversion::convert_transaction_to_typed_transaction;
use ethers::types::{Transaction, U256};
use ethers::utils::format_units;
use ethers::{providers::Middleware, types::Address};
use log::{error, info};

use super::setup::MainnetWallet;

impl MainnetWallet {
    pub async fn simulate_buying_token_for_weth(&self, token: &Erc20Token) -> anyhow::Result<U256> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        let mut new_token_balance = U256::from(0);
        let router = UNISWAP_V2_ROUTER::new(router_address, self.client.clone());

        println!("........................................................");
        self.get_wallet_eth_balance().await?;
        let amount_in = amount_of_token_to_purchase()?;
        println!("buying {}", token.name);

        // calculate amount amount out and gas used
        println!("........................................................");
        let amount_out_min = get_amount_out_uniswap_v2(
            weth_address,
            token.address,
            amount_in,
            TxSlippage::TwoPercent,
            &self.client,
        )
        .await?;

        let amount_out_min_readable = format_units(amount_out_min, 18u32)?;
        println!("calculated amount out min {}", amount_out_min_readable);
        println!("........................................................");

        let deadline = self.get_current_timestamp().await?;
        let deadline = deadline + 300; //  add 5 mins

        // Call Uniswap V2 swapExactTokensForTokens
        // Note: Ensure token_in has been approved for the router if it's not WETH
        // Already done in prepare_account or before this call as needed
        let tx = router
            .swap_exact_eth_for_tokens(
                amount_out_min,
                vec![weth_address, token.address],
                self.sender,
                U256::from(deadline),
            )
            .value(amount_in)
            .gas(U256::from(400_000));

        // sent transaction
        info!("sending tx");
        let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // wait for transaction receipt
                info!("awaiting tx receipt");
                let receipt = pending_tx.await?.unwrap();

                // gas update
                println!("updating gas cost");
                update_tx_gas_cost_data(&receipt, &token).await?;

                let tx_hash = receipt.transaction_hash;

                // self.trace_transaction(tx_hash).await?;

                println!("........................................................");
                println!("balance after buying {}...", token.name);
                new_token_balance = self.get_wallet_token_balance(token.address).await?;
                self.get_wallet_eth_balance().await?;
                println!("........................................................");
            }
            Err(tx_err) => {
                // Sending the transaction failed
                error!("Failed to send transaction: {:?}", tx_err);
            }
        }

        Ok(new_token_balance)
    }

    pub async fn simulate_selling_token_for_weth(
        &self,
        token: &Erc20Token,
    ) -> anyhow::Result<U256> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let token_contract = ERC20::new(token.address, self.client.clone());

        let mut new_token_balance = U256::from(0);
        let router = UNISWAP_V2_ROUTER::new(router_address, self.client.clone());

        println!("........................................................");
        self.get_wallet_eth_balance().await?;
        let amount_to_sell = self.get_wallet_token_balance(token.address).await?;

        //approve swap router to trade token
        token_contract
            .approve(router_address, amount_to_sell)
            .gas(U256::from(150_000))
            .send()
            .await?;

        println!("........................................................");
        let amount_out_min = get_amount_out_uniswap_v2(
            token.address,
            weth_address,
            amount_to_sell,
            TxSlippage::TwoPercent,
            &self.client,
        )
        .await?;

        let amount_out_min_readable = format_units(amount_out_min, "ether")?;
        println!("calculated amount out min {}", amount_out_min_readable);
        println!("........................................................");

        let deadline = self.get_current_timestamp().await?;
        let deadline = deadline + 300; //  add 5 mins

        // Call Uniswap V2 swapExactTokensForTokens
        // Note: Ensure token_in has been approved for the router if it's not WETH
        // Already done in prepare_account or before this call as needed
        let tx = router.swap_exact_tokens_for_eth(
            amount_to_sell,
            amount_out_min,
            vec![token.address, weth_address],
            self.sender,
            U256::from(deadline),
        );

        info!("set gas limit for transaction");
        let tx = tx.gas(U256::from(400_000));

        // sent transaction
        info!("sending swap transcation");
        let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // wait for transaction receipt
                info!("awaiting transaction receipt");
                let receipt = pending_tx.await?.unwrap();

                // gas update
                update_tx_gas_cost_data(&receipt, &token).await?;

                let tx_hash = receipt.transaction_hash;

                // self.trace_transaction(tx_hash).await?;

                println!("........................................................");
                println!("balance AFTER to selling {}", token.name);
                new_token_balance = self.get_wallet_token_balance(token.address).await?;
                self.get_wallet_eth_balance().await?;
                println!("........................................................");
                println!("........................................................");
                self.get_current_profit_loss().await?;
                println!("........................................................");
                println!("........................................................");
            }
            Err(tx_err) => {
                // Sending the transaction failed
                error!("Failed to send transaction: {:?}", tx_err);
            }
        }

        Ok(new_token_balance)
    }
}
