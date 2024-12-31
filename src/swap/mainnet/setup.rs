use crate::{
    abi::erc20::ERC20,
    data::contracts::{CHAIN, CONTRACT},
    utils::{
        tx::{calculate_next_block_base_fee, get_current_block, get_wallet},
        type_conversion::u256_to_f64_with_decimals,
    },
};
use anyhow::Result;
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{Signer, Wallet},
    types::{Address, U256},
    utils::format_units,
};
use std::sync::Arc;

pub struct TxWallet {
    pub signed_client: Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>,
    pub client: Arc<Provider<Ws>>,
    pub wallet: Wallet<SigningKey>,
    pub sender: Address,
    pub starting_eth_balance: U256,
}

impl TxWallet {
    pub async fn new() -> Result<Self> {
        // setup websocket connect to eth node
        // let ws_url = CONTRACT.get_address().ws_url.clone();
        // TODO - switch to ws_url once eth node up
        let ws_url = CONTRACT.get_address().alchemy_url.clone();
        let provider = Provider::<Ws>::connect(ws_url.clone()).await?;
        let client = Arc::new(provider.clone());

        // wallet config and address
        let wallet = get_wallet()?;
        let sender = wallet.address();

        // setup signed client
        let signer_middleware = SignerMiddleware::new(provider, wallet.clone());

        let signed_client = Arc::new(signer_middleware);

        let mut simulator = Self {
            signed_client,
            client: client.clone(),
            wallet,
            sender,
            starting_eth_balance: U256::zero(),
        };

        let starting_balance = simulator.get_wallet_eth_balance().await?;

        simulator = Self {
            starting_eth_balance: starting_balance,
            ..simulator
        };

        Ok(simulator)
    }

    // DUPS setup trait
    pub async fn get_wallet_token_balance(&self, token_address: Address) -> anyhow::Result<U256> {
        let token_contract = ERC20::new(token_address, self.client.clone());

        let token_balance = token_contract.balance_of(self.sender).await?;

        Ok(token_balance)
    }

    pub async fn get_wallet_eth_balance(&self) -> anyhow::Result<U256> {
        // get account balance to see how much of new token recieved

        let new_eth_balance_u256 = self.client.get_balance(self.sender, None).await?;
        let eth_balance = format_units(new_eth_balance_u256, "ether")?;

        println!("YOU HAVE {} of ETH", eth_balance);
        Ok(new_eth_balance_u256)
    }

    pub async fn get_current_timestamp(&self) -> anyhow::Result<u64> {
        // Get current block timestamp for deadline
        let current_block = self.client.get_block_number().await?;
        let current_block_details = self.client.get_block(current_block).await?;
        let current_timestamp = current_block_details
            .ok_or_else(|| anyhow::anyhow!("No current block details"))?
            .timestamp
            .as_u64();

        Ok(current_timestamp)
    }

    // TODO - FIX -> ether balance should be saved in global state
    pub async fn get_current_profit_loss(&self) -> anyhow::Result<()> {
        let eth_balance = self.client.get_balance(self.sender, None).await?;
        let profit = eth_balance - self.starting_eth_balance;
        let profit = format_units(profit, "ether")?;

        println!("CURRENT PROFIT IS {}", profit);

        Ok(())
    }

    pub async fn get_gas_and_priority_fee(&self) -> anyhow::Result<(U256, U256)> {
        let (block, _) = get_current_block(&self.client).await?;

        let next_base_fee = calculate_next_block_base_fee(&block)?;

        let buffer = next_base_fee / 20; // 5% buffer
        let adjusted_max_fee = next_base_fee + buffer;
        let prority_max_fee = adjusted_max_fee / 10; // 10% suggested priority fee
        Ok((adjusted_max_fee, prority_max_fee))
    }
}
