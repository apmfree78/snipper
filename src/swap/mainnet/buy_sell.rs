use crate::data::gas::update_tx_gas_cost_data;
use crate::data::tokens::Erc20Token;
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
use log::{error, info, warn};

use super::setup::TxWallet;

impl TxWallet {
    pub async fn buy_token_for_eth(&self, token: &Erc20Token) -> anyhow::Result<U256> {
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

                        let _ = receipt.transaction_hash;

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

        // sent transaction
        info!("sending swap transcation");
        let pending_tx_result = self
            .signed_client
            .send_transaction(TypedTransaction::Eip1559(uniswap_swap_tx), None)
            .await;
        // let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // wait for transaction receipt
                info!("awaiting transaction receipt");
                let receipt = pending_tx.await?.unwrap();

                // gas update
                update_tx_gas_cost_data(&receipt, &token).await?;

                let _ = receipt.transaction_hash;

                // self.trace_transaction(tx_hash).await?;

                println!("........................................................");
                new_token_balance = self
                    .get_wallet_token_balance_by_address(token.address)
                    .await?;
                println!(
                    "{} balance AFTER to selling {}",
                    token.name, new_token_balance
                );
                self.get_wallet_eth_balance().await?;
                println!("........................................................");
                println!("........................................................");
                // self.get_current_profit_loss().await?;
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
