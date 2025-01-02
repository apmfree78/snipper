use crate::abi::erc20::ERC20;
use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::data::contracts::{CHAIN, CONTRACT};
use crate::data::tokens::Erc20Token;
use crate::utils::tx::{
    calculate_next_block_base_fee, get_approval_calldata, get_current_block, TxSlippage,
};
use async_trait::async_trait;
use ethers::types::{Address, Block, H256, U256};
use ethers::utils::format_units;
use ethers::{
    core::k256::ecdsa::SigningKey,
    core::types::Chain,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{Signer, Wallet},
    types::{Eip1559TransactionRequest, NameOrAddress},
};
use log::info;
use std::sync::Arc;

use super::anvil::simlator::AnvilSimulator;
use super::mainnet::setup::TxWallet;

#[async_trait]
pub trait Txs {
    fn signed_client(&self) -> Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>;
    fn client(&self) -> Arc<Provider<Ws>>;
    fn sender(&self) -> Address;

    async fn get_current_timestamp(&self) -> anyhow::Result<u64> {
        // Get current block timestamp for deadline
        let current_block = self.signed_client().get_block_number().await?;
        let current_block_details = self.signed_client().get_block(current_block).await?;
        let current_timestamp = current_block_details
            .ok_or_else(|| anyhow::anyhow!("No current block details"))?
            .timestamp
            .as_u64();

        Ok(current_timestamp)
    }

    async fn get_wallet_token_balance(&self, token: &Erc20Token) -> anyhow::Result<U256> {
        let new_token_balance_u256 = self
            .get_wallet_token_balance_by_address(token.address)
            .await?;
        let token_balance = format_units(new_token_balance_u256, u32::from(token.decimals))?;
        println!(
            "YOU HAVE {} of {}, ({})",
            token_balance, token.name, token.symbol
        );
        Ok(new_token_balance_u256)
    }

    async fn get_wallet_token_balance_by_address(
        &self,
        token_address: Address,
    ) -> anyhow::Result<U256> {
        // get account balance to see how much of new token recieved
        info!("getting token balance");
        let token_contract = ERC20::new(token_address, self.signed_client());

        let new_token_balance_u256 = token_contract.balance_of(self.sender()).call().await?;

        Ok(new_token_balance_u256)
    }

    async fn get_wallet_eth_balance(&self) -> anyhow::Result<U256> {
        // get account balance to see how much of new token recieved

        let new_eth_balance_u256 = self
            .signed_client()
            .get_balance(self.sender(), None)
            .await?;
        let eth_balance = format_units(new_eth_balance_u256, 18u32)?;

        println!("YOU HAVE {} of ETH", eth_balance);
        Ok(new_eth_balance_u256)
    }

    async fn get_gas_and_priority_fee(&self) -> anyhow::Result<(U256, U256)> {
        let (block, _) = get_current_block(&self.client()).await?;

        let next_base_fee = calculate_next_block_base_fee(&block)?;

        let buffer = next_base_fee / 20; // 5% buffer
        let adjusted_max_fee = next_base_fee + buffer;
        let prority_max_fee = adjusted_max_fee / 10; // 10% suggested priority fee
        Ok((adjusted_max_fee, prority_max_fee))
    }
}

impl Txs for AnvilSimulator {
    fn signed_client(&self) -> Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>> {
        self.signed_client.clone()
    }
    fn client(&self) -> Arc<Provider<Ws>> {
        self.client.clone()
    }
    fn sender(&self) -> Address {
        self.sender
    }
}

impl Txs for TxWallet {
    fn signed_client(&self) -> Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>> {
        self.signed_client.clone()
    }
    fn client(&self) -> Arc<Provider<Ws>> {
        self.client.clone()
    }
    fn sender(&self) -> Address {
        self.sender
    }
}
