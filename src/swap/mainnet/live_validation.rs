use std::time::Duration;

use crate::swap::anvil::validation::TokenStatus;
use crate::swap::mainnet::setup::TxType;
use crate::swap::tx_trait::Txs;
use crate::{abi::erc20::ERC20, data::tokens::Erc20Token};
use ethers::types::{Address, U256};
use tokio::time::sleep;

use super::setup::{TxWallet, WalletType};

impl TxWallet {
    /// Takes a snapshot of the current blockchain state using anvil
    pub async fn validate_with_live_buy_sell(
        &self,
        token: &Erc20Token,
        wallet_type: WalletType,
    ) -> anyhow::Result<TokenStatus> {
        // launch new anvil node for validation

        println!("validating token...");

        // Try to buy the token
        // let balance_before = anvil.get_token_balance(token).await?;
        println!("simulate buying token for validation");
        // TODO - UPDATE TO SET TOKEN PURCHASE AMOUNT
        let token_balance = self
            .buy_tokens_for_eth(token, wallet_type.clone(), TxType::Test)
            .await?;

        println!("check token balance after purchase");
        if token_balance == U256::from(0) {
            println!("No tokens received after buy, reverting...");
            // revert if something suspicious
            return Ok(TokenStatus::CannotBuy);
        }

        // Increase time by 300 seconds (5 minutes)
        // println!("simulating time elapse of 5 mins");
        // anvil
        //     .signed_client
        //     .provider()
        //     .request::<_, u64>("evm_increaseTime", [3000u64])
        //     .await?;

        // println!("check token balance after five minutes");
        // check token balance did not drop after time elapse
        let balance_after_buy = self
            .get_wallet_token_balance_by_address(token.address)
            .await?;
        if balance_after_buy < token_balance {
            println!("Token are dropping or going to zero after 5 mins...");
            // revert if something suspicious
            return Ok(TokenStatus::CannotBuy);
        }
        // 3. Sleep for 15 seconds
        println!("Waiting 15 seconds before selling...");
        sleep(Duration::from_secs(15)).await;
        //
        // self.do_transfer_test(token.address, balance_after_buy)
        //     .await?;
        // // // simulate transfer between wallets
        // // self
        // //     .do_dummy_transfer(token.address, balance_after_buy)
        // //     .await?;

        println!("check token balance after 15 secs");
        // check token balance did not drop after time elapse
        let balance_after_transfer = self
            .get_wallet_token_balance_by_address(token.address)
            .await?;
        if balance_after_transfer < token_balance {
            println!("Token are dropping or going to zero after transfer...");
            // revert if something suspicious
            return Ok(TokenStatus::CannotBuy);
        }

        // Now attempt to sell
        // info!("simulate selling token for validation");
        let eth_recieved = self.sell_token_for_eth(token, wallet_type).await?;

        println!("check token balance after sale (should be zero)");
        let balance_after_sell = self
            .get_wallet_token_balance_by_address(token.address)
            .await?;
        if balance_after_sell != U256::from(0) || eth_recieved == U256::zero() {
            println!("cannot sell {}, scam alert", token.name);
            // If you must revert because the sale is unsuccessful, do it here
            return Ok(TokenStatus::CannotSell);
        }

        println!("{} is legit", token.name);
        Ok(TokenStatus::Legit)
    }

    pub async fn do_transfer_test(
        &self,
        token_address: Address,
        amount: U256,
    ) -> anyhow::Result<()> {
        // 1) Use the ERC20::new(...) constructor
        let wallet_2 = if self.type_of == WalletType::Main {
            TxWallet::new(WalletType::Test).await?
        } else {
            TxWallet::new(WalletType::Main).await?
        };

        let token_contract_wallet_1 = ERC20::new(token_address, self.signed_client.clone());
        let token_contract_wallet_2 = ERC20::new(token_address, wallet_2.signed_client.clone());

        // 2) Transfer from wallet1 -> wallet2
        token_contract_wallet_1
            .transfer(wallet_2.signed_client.address(), amount)
            .send()
            .await?;

        // 3) Transfer from wallet2 -> wallet1
        token_contract_wallet_2
            .transfer(self.signed_client.address(), amount)
            .send()
            .await?;

        Ok(())
    }
}
