use crate::abi::erc20::ERC20;
use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::app_config::{AppMode, APP_MODE};
use crate::data::contracts::CONTRACT;
use crate::data::gas::update_tx_gas_cost_data;
use crate::data::tokens::Erc20Token;
use crate::swap::tx_trait::Txs;
use crate::utils::tx::{get_amount_out_uniswap_v2, test_amount_of_token_to_purchase, TxSlippage};
use crate::utils::type_conversion::convert_transaction_to_typed_transaction;
use ethers::types::{Transaction, U256};
use ethers::{providers::Middleware, types::Address};
use log::error;

use super::simlator::AnvilSimulator;

impl AnvilSimulator {
    // function to simulate mempool tx
    pub async fn add_liquidity_eth(&self, mempool_tx: &Transaction) -> anyhow::Result<()> {
        // let sender_address = mempool_tx.from;
        // self.signed_client
        //     .provider()
        //     .request::<_, ()>("anvil_impersonateAccount", [sender_address])
        //     .await?;

        // Convert and send the first transaction
        let mempool_tx_typed = convert_transaction_to_typed_transaction(&mempool_tx);

        // println!("calculating oracle update on anvil");
        // Send the transaction and get the PendingTransaction
        let pending_tx = self
            .signed_client
            .send_transaction(mempool_tx_typed, None)
            .await?;

        // Await the transaction receipt immediately to avoid capturing `pending_tx` in the async state
        let _receipt = pending_tx.await?;
        // println!("add liquidity eth complete!");

        // // Stop impersonating the account
        // self.signed_client
        //     .provider()
        //     .request::<_, ()>("anvil_stopImpersonatingAccount", [sender_address])
        //     .await?;

        Ok(())
    }

    pub async fn simulate_buying_token_for_weth(&self, token: &Erc20Token) -> anyhow::Result<U256> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        let mut new_token_balance = U256::from(0);
        let router = UNISWAP_V2_ROUTER::new(router_address, self.signed_client.clone());

        // Impersonate the account you want to send the transaction from
        // self.signed_client
        //     .provider()
        //     .request::<_, ()>("anvil_impersonateAccount", [self.sender])
        //     .await?;

        // println!("........................................................");
        self.get_wallet_eth_balance().await?;
        let amount_in = test_amount_of_token_to_purchase()?;
        // println!("buying {}", token.name);

        // calculate amount amount out and gas used
        // println!("........................................................");
        let amount_out_min = get_amount_out_uniswap_v2(
            weth_address,
            token.address,
            amount_in,
            TxSlippage::TenPercent,
            &self.client,
        )
        .await?;

        // let amount_out_min_readable = format_units(amount_out_min, 18u32)?;
        // println!("calculated amount out min {}", amount_out_min_readable);
        // println!("........................................................");

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
            .gas(U256::from(300_000));

        // sent transaction
        // info!("sending tx");
        let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // Transaction sent successfully
                // println!("Transaction sent, awaiting receipt");
                // let tx_hash = pending_tx.tx_hash();
                // debug!("tx_hash => {:?}", tx_hash);

                // wait for transaction receipt
                // info!("awaiting tx receipt");
                let receipt = pending_tx.await?.unwrap();

                // gas update
                // println!("updating gas cost");
                update_tx_gas_cost_data(&receipt, &token).await?;

                // let tx_hash = receipt.transaction_hash;

                // self.trace_transaction(tx_hash).await?;

                // println!("........................................................");
                // println!("balance after buying {}...", token.name);
                new_token_balance = self
                    .get_wallet_token_balance_by_address(token.address)
                    .await?;
                self.get_wallet_eth_balance().await?;
                // println!("........................................................");
            }
            Err(tx_err) => {
                // Sending the transaction failed
                error!("Failed to send transaction: {:?}", tx_err);

                // Try to extract more information from the error
                // if let Some(revert_reason) = extract_revert_reason(&tx_err) {
                //     error!("Revert reason: {}", revert_reason);
                // } else {
                //     error!("Failed to extract revert reason");
                // }
            }
        }

        // Stop impersonating the account after the transaction is complete
        // self.signed_client
        //     .provider()
        //     .request::<_, ()>("anvil_stopImpersonatingAccount", [self.sender])
        //     .await?;
        Ok(new_token_balance)
    }

    pub async fn simulate_selling_token_for_weth(
        &self,
        token: &Erc20Token,
    ) -> anyhow::Result<U256> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let token_contract = ERC20::new(token.address, self.signed_client.clone());

        let mut new_token_balance = U256::from(0);
        let router = UNISWAP_V2_ROUTER::new(router_address, self.signed_client.clone());

        // Impersonate the account you want to send the transaction from
        // self.signed_client
        //     .provider()
        //     .request::<_, ()>("anvil_impersonateAccount", [self.sender])
        //     .await?;

        // self.show_eth_uniswap_v2_pair(&token).await?;

        // println!("........................................................");
        self.get_wallet_eth_balance().await?;
        let amount_to_sell = self
            .get_wallet_token_balance_by_address(token.address)
            .await?;

        //approve swap router to trade token
        token_contract
            .approve(router_address, amount_to_sell)
            .send()
            .await?;

        // println!("........................................................");
        let amount_out_min = get_amount_out_uniswap_v2(
            token.address,
            weth_address,
            amount_to_sell,
            TxSlippage::TenPercent,
            &self.client,
        )
        .await?;

        // let amount_out_min_readable = format_units(amount_out_min, 18u32)?;
        // println!("calculated amount out min {}", amount_out_min_readable);
        // println!("........................................................");

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

        // info!("set gas limit for transaction");
        let tx = tx.gas(U256::from(300_000));

        // sent transaction
        // info!("sending swap transcation");
        let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // Transaction sent successfully
                // info!("Transaction sent, awaiting receipt");
                // let tx_hash = pending_tx.tx_hash();
                // debug!("tx_hash => {:?}", tx_hash);

                // wait for transaction receipt
                // info!("awaiting transaction receipt");
                let receipt = pending_tx.await?.unwrap();

                // gas update only for simulation, otherwise get gas from
                // actual transaction
                if APP_MODE == AppMode::Simulation {
                    update_tx_gas_cost_data(&receipt, &token).await?;
                }

                // let tx_hash = receipt.transaction_hash;
                //
                // self.trace_transaction(tx_hash).await?;

                // println!("........................................................");
                // println!("balance AFTER to selling {}", token.name);
                new_token_balance = self
                    .get_wallet_token_balance_by_address(token.address)
                    .await?;
                self.get_wallet_eth_balance().await?;
            }
            Err(tx_err) => {
                // Sending the transaction failed
                error!("Failed to send transaction: {:?}", tx_err);

                // Try to extract more information from the error
                // if let Some(revert_reason) = extract_revert_reason(&tx_err) {
                //     error!("Revert reason: {}", revert_reason);
                // } else {
                //     error!("Failed to extract revert reason");
                // }
            }
        }

        // Stop impersonating the account after the transaction is complete
        // self.signed_client
        //     .provider()
        //     .request::<_, ()>("anvil_stopImpersonatingAccount", [self.sender])
        //     .await?;
        Ok(new_token_balance)
    }
}
