use crate::data::gas::update_tx_gas_cost_data;
use crate::data::tokens::Erc20Token;
use crate::events::{decode_pair_created_event, parse_swap_receipt_logs_to_get_eth_amount_out};
use crate::swap::prepare_tx::{prepare_token_approval_tx, prepare_uniswap_swap_tx};
use crate::swap::tx_trait::Txs;
use crate::utils::tx::{
    amount_of_token_to_purchase, get_current_block, get_swap_exact_eth_for_tokens_calldata,
    get_swap_exact_tokens_for_eth_calldata, get_wallet_nonce,
};
use ethers::providers::Middleware;
use ethers::signers::Signer;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::U256;
use ethers::utils::format_units;
use log::{error, info, warn};

use super::setup::TxWallet;

impl TxWallet {
    pub async fn buy_tokens_for_eth(&self, token: &Erc20Token) -> anyhow::Result<U256> {
        let (block, _) = get_current_block(&self.client).await?;
        let mut new_token_balance = U256::from(0);

        println!("........................................................");
        self.get_wallet_eth_balance().await?;
        println!("buying {}", token.name);

        println!("getting calldata...");
        // FOR swap exact eth ONLY
        // encode the call data
        let calldata = get_swap_exact_eth_for_tokens_calldata(
            &token,
            self.wallet.address(),
            block.timestamp.as_u32(),
            &self.client,
        )
        .await?;

        println!("getting amount of token to purchase...");
        // FOR swap exact eth ONLY
        let eth_to_send_with_tx = amount_of_token_to_purchase()?;

        let nonce = get_wallet_nonce(self.wallet.address(), &self.client).await?;
        println!("nonce for purchase tx => {}", nonce);

        println!("prepaparing tranasaction...");
        let (uniswap_swap_tx, _) =
            prepare_uniswap_swap_tx(calldata, eth_to_send_with_tx, &block, nonce)?;

        // sent transaction
        println!("sending swap transcation");
        let pending_tx_result = self
            .signed_client
            .send_transaction(TypedTransaction::Eip1559(uniswap_swap_tx), None)
            .await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // wait for transaction receipt
                println!("awaiting tx receipt");
                match pending_tx.await? {
                    Some(receipt) => {
                        // gas update
                        println!("updating gas cost");
                        update_tx_gas_cost_data(&receipt, &token).await?;

                        // let _ = receipt.transaction_hash;

                        // self.trace_transaction(tx_hash).await?;
                    }
                    None => warn!("no reciept for transaction"),
                };

                println!("........................................................");
                new_token_balance = self
                    .get_wallet_token_balance_by_address(token.address)
                    .await?;
                println!(
                    "{} balance after buying {}...",
                    new_token_balance, token.name
                );
                self.get_wallet_eth_balance().await?;
                println!("........................................................");
            }
            Err(tx_err) => {
                // Sending the transaction failed
                error!("Failed to send transaction: {:?}", tx_err);
                println!("Failed to send transaction: {:?}", tx_err);
            }
        }

        Ok(new_token_balance)
    }

    pub async fn sell_token_for_eth(&self, token: &Erc20Token) -> anyhow::Result<U256> {
        let mut new_token_balance = U256::from(0);
        let (block, _) = get_current_block(&self.client).await?;

        println!("........................................................");
        self.get_wallet_eth_balance().await?;
        let amount_to_sell = self
            .get_wallet_token_balance_by_address(token.address)
            .await?;

        //  Get nonce
        let mut nonce = get_wallet_nonce(self.wallet.address(), &self.client).await?;
        println!("nonce for approval tx => {}", nonce);

        println!("preparing approval tx...");
        let approval_tx =
            prepare_token_approval_tx(&token, amount_to_sell, &block, nonce, &self.client)?;

        info!("sending approval transcation");
        let pending_approval = self
            .signed_client
            .send_transaction(approval_tx, None)
            .await?;

        let receipt = pending_approval.await?;
        match receipt {
            Some(_) => println!("approval successful!"),
            None => panic!("could not approve token"),
        }

        println!("iterate nonce for swap tx...");
        nonce += U256::from(1);
        println!("nonce for swap tx => {}", nonce);

        let token_swap_calldata = get_swap_exact_tokens_for_eth_calldata(
            &token,
            self.wallet.address(),
            amount_to_sell,
            block.timestamp.as_u32(),
            &self.client,
        )
        .await?;

        let (uniswap_swap_tx, _) =
            prepare_uniswap_swap_tx(token_swap_calldata, U256::zero(), &block, nonce)?;

        // get eth balane before token sale

        // sent transaction
        info!("sending swap transcation");
        let pending_tx_result = self
            .signed_client
            .send_transaction(TypedTransaction::Eip1559(uniswap_swap_tx), None)
            .await;
        // let pending_tx_result = tx.send().await;

        let eth_recieved_from_sale = match pending_tx_result {
            Ok(pending_tx) => {
                // wait for transaction receipt
                info!("awaiting transaction receipt");
                let eth_from_sale = match pending_tx.await? {
                    Some(receipt) => {
                        // gas update
                        update_tx_gas_cost_data(&receipt, &token).await?;

                        let amount_out =
                            parse_swap_receipt_logs_to_get_eth_amount_out(&receipt, token)?;
                        amount_out
                    }
                    None => {
                        warn!("could not get transaction receipt ");
                        U256::zero()
                    }
                };

                // let _ = receipt.transaction_hash;

                // self.trace_transaction(tx_hash).await?;
                let eth_recieved = format_units(eth_from_sale, "ether")?;

                println!("........................................................");
                new_token_balance = self
                    .get_wallet_token_balance_by_address(token.address)
                    .await?;
                println!("{} eth recieved from sale of {}", eth_recieved, token.name);
                println!("{} of {} remaining", new_token_balance, token.name);
                // self.get_wallet_eth_balance().await?;
                println!("........................................................");
                println!("........................................................");
                // self.get_current_profit_loss().await?;
                println!("........................................................");
                println!("........................................................");

                // return amount of eth recieved from sale
                eth_from_sale
            }
            Err(tx_err) => {
                // Sending the transaction failed
                error!("Failed to send transaction: {:?}", tx_err);
                U256::zero()
            }
        };

        Ok(eth_recieved_from_sale)
    }
}
