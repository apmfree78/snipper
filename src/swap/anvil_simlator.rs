use crate::abi::erc20::ERC20;
use crate::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::data::contracts::{CHAIN, CONTRACT};
use crate::data::tokens::Erc20Token;
use crate::utils::type_conversion::{
    address_to_string, get_function_selector, u256_to_f64_with_decimals,
};
use anyhow::Result;
use ethers::types::{
    CallFrame, GethDebugTracerType, GethDebugTracingOptions, GethTrace, GethTraceFrame,
    TransactionRequest, H256, U256,
};
use ethers::utils::format_units;
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{Signer, Wallet},
    types::Address,
    utils::{Anvil, AnvilInstance},
};
use log::{debug, error, info};
use std::sync::Arc;

pub const STARTING_BALANCE: f64 = 1000.0;

#[derive(Debug, Clone, Copy)]
pub enum UniswapVersion {
    V2,
    V3,
}

pub struct AnvilSimulator {
    pub client: Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>,
    pub anvil: AnvilInstance,
    pub from_address: Address,
}

impl AnvilSimulator {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        // Main network provider   // Configure Anvil with forking
        let anvil = Anvil::new()
            .fork(rpc_url) // URL of your Geth node
            .spawn();

        // setup mock sender
        let from_address: Address = anvil.addresses()[0];
        let private_keys = anvil.keys();
        let from_private_key = private_keys[0].clone();

        // Connect to Anvil
        let anvil_ws_url = anvil.ws_endpoint();
        let provider = Provider::<Ws>::connect(anvil_ws_url).await?;

        // Create a wallet with the private key
        let wallet = Wallet::from(from_private_key).with_chain_id(CHAIN);

        // Create the SignerMiddleware
        let client = Arc::new(SignerMiddleware::new(provider, wallet));

        client
            .provider()
            .request::<_, ()>(
                "anvil_setBalance",
                [
                    format!("{:#x}", from_address),
                    "0x3635c9adc5dea00000".to_string(), //100 ETH
                ],
            )
            .await?;

        let simulator = Self {
            client,
            anvil,
            from_address,
        };

        simulator.prepare_account().await?;

        Ok(simulator)
    }

    /// Prepares the test account by converting some ETH to WETH and approving the router.
    pub async fn prepare_account(&self) -> anyhow::Result<()> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let weth_contract = ERC20::new(weth_address, self.client.clone());

        // Impersonate the from_address to send transactions from it
        self.client
            .provider()
            .request::<_, ()>("anvil_impersonateAccount", [self.from_address])
            .await?;

        // Convert 10 ETH to WETH using wrapETH method on the router
        let wrap_amount = ethers::utils::parse_ether("30.0")?;

        let eth_balance = self.client.get_balance(self.from_address, None).await?;
        debug!("ETH Balance of from_address: {}", eth_balance);
        debug!("amount of eth to wrap => {}", wrap_amount);

        if eth_balance < wrap_amount {
            return Err(anyhow::anyhow!(
                "Insufficient ETH balance: required {}, available {}",
                wrap_amount,
                eth_balance
            ));
        }

        let deposit_selector = get_function_selector("deposit()");

        let gas_price = self.client.provider().get_gas_price().await?;
        debug!("Current gas price: {}", gas_price);

        // Wrap 10 ETH into WETH
        let wrap_tx = self
            .client
            .provider()
            .send_transaction(
                TransactionRequest::new()
                    .to(weth_address)
                    .data(deposit_selector)
                    .value(wrap_amount)
                    .gas_price(gas_price)
                    .gas(U256::from(300_000)),
                None,
            )
            .await?;

        let _wrap_receipt = wrap_tx
            .await?
            .ok_or_else(|| anyhow::anyhow!("No wrapETH receipt received"))?;

        let approval_amount = ethers::utils::parse_ether("30")?;
        weth_contract
            .approve(router_address, approval_amount)
            .send()
            .await?;

        let weth_balance = weth_contract.balance_of(self.from_address).call().await?;
        let allowance = weth_contract
            .allowance(self.from_address, router_address)
            .call()
            .await?;
        debug!("WETH Balance of anvil mock account: {}", weth_balance);
        debug!("Allowance of anvil mock account:  {}", allowance);

        // Stop impersonating the account
        self.client
            .provider()
            .request::<_, ()>("anvil_stopImpersonatingAccount", [self.from_address])
            .await?;

        Ok(())
    }

    pub async fn simulate_buying_token_for_weth(&self, token: &Erc20Token) -> Result<U256> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        let mut new_token_balance = U256::from(0);
        let router = UNISWAP_V2_ROUTER::new(router_address, self.client.clone());
        // Impersonate the account you want to send the transaction from
        self.client
            .provider()
            .request::<_, ()>("anvil_impersonateAccount", [self.from_address])
            .await?;

        println!("........................................................");
        self.get_weth_balance().await?;
        self.get_eth_balance().await?;
        let amount_to_buy =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        println!("buying {} WETH of {}", amount_to_buy, token.name);
        let amount_in = ethers::utils::parse_ether(amount_to_buy)?;

        // calculate amount amount out and gas used
        println!("........................................................");
        let amount_out_min = self
            .get_amount_out_uniswap_v2(weth_address, token.address, amount_in)
            .await?;

        let amount_out_min_readable = format_units(amount_out_min, 18u32)?;
        println!("calculated amount out min {}", amount_out_min_readable);
        println!("........................................................");

        let deadline = self.get_current_timestamp().await?;
        let deadline = deadline + 300; //  add 5 mins

        // Call Uniswap V2 swapExactTokensForTokens
        // Note: Ensure token_in has been approved for the router if it's not WETH
        // Already done in prepare_account or before this call as needed
        let tx = router.swap_exact_tokens_for_tokens(
            amount_in,
            amount_out_min,
            vec![weth_address, token.address],
            self.from_address,
            U256::from(deadline),
        );

        info!("set gas limit for transaction");
        let tx = tx.gas(U256::from(300_000));

        // sent transaction
        info!("sending liquidate transcation");
        let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // Transaction sent successfully
                // info!("Transaction sent, awaiting receipt");
                // let tx_hash = pending_tx.tx_hash();
                // debug!("tx_hash => {:?}", tx_hash);

                // wait for transaction receipt
                info!("awaiting transaction receipt");
                let receipt = pending_tx.await?.unwrap();
                // info!("transaction receipt obtained ==> {:#?}", receipt);

                let tx_hash = receipt.transaction_hash;

                self.trace_transaction(tx_hash).await?;

                println!("........................................................");
                println!("balance after buying {}...", token.name);
                new_token_balance = self.get_token_balance(&token).await?;
                self.get_weth_balance().await?;
                self.get_eth_balance().await?;
                println!("........................................................");
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
        self.client
            .provider()
            .request::<_, ()>("anvil_stopImpersonatingAccount", [self.from_address])
            .await?;
        Ok(new_token_balance)
    }

    pub async fn simulate_selling_token_for_weth(&self, token: &Erc20Token) -> Result<U256> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let token_contract = ERC20::new(token.address, self.client.clone());

        let mut new_token_balance = U256::from(0);
        let router = UNISWAP_V2_ROUTER::new(router_address, self.client.clone());

        // Impersonate the account you want to send the transaction from
        self.client
            .provider()
            .request::<_, ()>("anvil_impersonateAccount", [self.from_address])
            .await?;

        self.show_weth_allowance_balance_sender_and_pair(&token)
            .await?;

        println!("........................................................");
        self.get_weth_balance().await?;
        self.get_eth_balance().await?;
        let amount_to_sell = self.get_token_balance(&token).await?;

        //approve swap router to trade token
        token_contract
            .approve(router_address, amount_to_sell)
            .send()
            .await?;

        println!("........................................................");
        let amount_out_min = self
            .get_amount_out_uniswap_v2(token.address, weth_address, amount_to_sell)
            .await?;

        let amount_out_min_readable = format_units(amount_out_min, 18u32)?;
        println!("calculated amount out min {}", amount_out_min_readable);
        println!("........................................................");

        let deadline = self.get_current_timestamp().await?;
        let deadline = deadline + 300; //  add 5 mins

        // Call Uniswap V2 swapExactTokensForTokens
        // Note: Ensure token_in has been approved for the router if it's not WETH
        // Already done in prepare_account or before this call as needed
        let tx = router.swap_exact_tokens_for_tokens(
            amount_to_sell,
            amount_out_min,
            vec![token.address, weth_address],
            self.from_address,
            U256::from(deadline),
        );

        info!("set gas limit for transaction");
        let tx = tx.gas(U256::from(300_000));

        // sent transaction
        info!("sending swap transcation");
        let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // Transaction sent successfully
                // info!("Transaction sent, awaiting receipt");
                // let tx_hash = pending_tx.tx_hash();
                // debug!("tx_hash => {:?}", tx_hash);

                // wait for transaction receipt
                info!("awaiting transaction receipt");
                let receipt = pending_tx.await?.unwrap();
                // info!("transaction receipt obtained ==> {:#?}", receipt);

                let tx_hash = receipt.transaction_hash;

                self.trace_transaction(tx_hash).await?;

                println!("........................................................");
                println!("balance AFTER to selling {}", token.name);
                new_token_balance = self.get_token_balance(&token).await?;
                self.get_weth_balance().await?;
                self.get_eth_balance().await?;
                println!("........................................................");
                println!("........................................................");
                self.get_current_profit_loss().await?;
                println!("........................................................");
                println!("........................................................");
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
        self.client
            .provider()
            .request::<_, ()>("anvil_stopImpersonatingAccount", [self.from_address])
            .await?;
        Ok(new_token_balance)
    }

    async fn get_amount_out_uniswap_v2(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> anyhow::Result<U256> {
        let uniswap_v2_router_address: Address =
            CONTRACT.get_address().uniswap_v2_router.parse()?;
        let router = UNISWAP_V2_ROUTER::new(uniswap_v2_router_address, self.client.clone());

        let amounts = router
            .get_amounts_out(amount_in, vec![token_in, token_out])
            .call()
            .await?;

        let amount_out = amounts[amounts.len() - 1];

        // reduce by 2% to account for token volatility
        let amount_out = amount_out * U256::from(98) / U256::from(100);

        Ok(amount_out)
    }

    async fn get_current_timestamp(&self) -> anyhow::Result<u64> {
        // Get current block timestamp for deadline
        let current_block = self.client.get_block_number().await?;
        let current_block_details = self.client.get_block(current_block).await?;
        let current_timestamp = current_block_details
            .ok_or_else(|| anyhow::anyhow!("No current block details"))?
            .timestamp
            .as_u64();

        Ok(current_timestamp)
    }

    // ***************** ***************** **************** **********************************
    // ***************** SUPPORTING METHODS FOR DEBUGGING AND DIAGNOSIS *****************
    // ***************** ***************** **************** **********************************

    async fn trace_transaction(&self, tx_hash: H256) -> Result<()> {
        let mut tracing_options = GethDebugTracingOptions::default();
        tracing_options.disable_storage = Some(false); // Enable storage tracing
        tracing_options.disable_stack = Some(false); // Enable stack tracing
        tracing_options.tracer = Some(GethDebugTracerType::BuiltInTracer(
            ethers::types::GethDebugBuiltInTracerType::CallTracer,
        ));

        let trace = self
            .client
            .provider()
            .debug_trace_transaction(tx_hash, tracing_options)
            .await?;

        println!("Transaction trace: {:?}", trace);

        match trace {
            GethTrace::Known(GethTraceFrame::CallTracer(ref call_frame)) => {
                if let Some(revert_call) = find_revert(call_frame) {
                    debug!("Revert: {:?}", revert_call);
                    println!("Revert occurred in call to: {:?}", revert_call.to);

                    // Proceed to decode the function
                } else {
                    println!("No revert found in the trace");
                }
            }
            _ => {
                println!("Unexpected trace format");
            }
        }
        Ok(())
    }

    async fn show_weth_allowance_balance_sender_and_pair(
        &self,
        token: &Erc20Token,
    ) -> anyhow::Result<()> {
        let factory_address: Address = CONTRACT.get_address().uniswap_v2_factory.parse()?;
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let weth_contract = ERC20::new(weth_address, self.client.clone());

        let weth_balance = weth_contract.balance_of(self.from_address).call().await?;
        let allowance = weth_contract
            .allowance(self.from_address, router_address)
            .call()
            .await?;
        debug!("WETH Balance of anvil mock account: {}", weth_balance);
        debug!("Allowance of anvil mock account:  {}", allowance);
        // debug!(
        //     "Transaction sender (self.from_address): {:?}",
        //     self.from_address
        // );

        let factory = UNISWAP_V2_FACTORY::new(factory_address, self.client.clone());

        if token.is_token_0 {
            let pair_address = factory.get_pair(token.address, weth_address).call().await?;
            debug!("pair address for WETH-{}: {:?}", token.name, pair_address);
        } else {
            let pair_address = factory.get_pair(weth_address, token.address).call().await?;
            debug!("pair address for WETH-{}: {:?}", token.name, pair_address);
        }

        let pair_address = address_to_string(token.pair_address);
        debug!(
            "REAL pair address for WETH-{}: {:?}",
            token.name, pair_address
        );

        Ok(())
    }

    pub async fn get_token_balance(&self, token: &Erc20Token) -> anyhow::Result<U256> {
        let new_token_balance_u256 = self.get_token_balance_by_address(token.address).await?;
        let token_balance = format_units(new_token_balance_u256, u32::from(token.decimals))?;
        println!(
            "YOU HAVE {} of {}, ({})",
            token_balance, token.name, token.symbol
        );
        Ok(new_token_balance_u256)
    }

    pub async fn get_token_balance_by_address(
        &self,
        token_address: Address,
    ) -> anyhow::Result<U256> {
        // get account balance to see how much of new token recieved
        let token_contract = ERC20::new(token_address, self.client.clone());

        let new_token_balance_u256 = token_contract.balance_of(self.from_address).call().await?;

        Ok(new_token_balance_u256)
    }

    pub async fn get_current_profit_loss(&self) -> anyhow::Result<()> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let eth_balance = self.client.get_balance(self.from_address, None).await?;
        let weth = ERC20::new(weth_address, self.client.clone());

        let weth_balance = weth.balance_of(self.from_address).call().await?;

        let total_balance = u256_to_f64_with_decimals(weth_balance + eth_balance, 18)?;

        let profit = total_balance - STARTING_BALANCE;

        println!("CURRENT PROFIT IS {}", profit);

        Ok(())
    }

    pub async fn get_eth_balance(&self) -> anyhow::Result<U256> {
        // get account balance to see how much of new token recieved

        let new_eth_balance_u256 = self.client.get_balance(self.from_address, None).await?;
        let eth_balance = format_units(new_eth_balance_u256, 18u32)?;

        println!("YOU HAVE {} of ETH", eth_balance);
        Ok(new_eth_balance_u256)
    }

    async fn get_weth_balance(&self) -> anyhow::Result<U256> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        // get account balance to see how much of new token recieved
        let token_contract = ERC20::new(weth_address, self.client.clone());

        let new_token_balance_u256 = token_contract.balance_of(self.from_address).call().await?;
        let token_balance = format_units(new_token_balance_u256, u32::from(18u32))?;

        println!("YOU HAVE {} of WETH", token_balance);
        Ok(new_token_balance_u256)
    }
}

fn find_revert(trace: &CallFrame) -> Option<&CallFrame> {
    // If this call frame has an error, and no further nested calls, it's the revert point
    if trace.error.is_some() && (trace.calls.is_none() || trace.calls.as_ref().unwrap().is_empty())
    {
        return Some(trace);
    }

    // If there are nested calls, check them recursively
    if let Some(calls) = &trace.calls {
        for call in calls {
            if let Some(revert_call) = find_revert(call) {
                return Some(revert_call);
            }
        }
    }

    None
}
