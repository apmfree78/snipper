use crate::abi::erc20::ERC20;
use crate::abi::uniswap_v3_router::{
    ExactInputSingleParams, UNISWAP_V3_ROUTER, UNISWAP_V3_ROUTER_ABI,
};
use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use anyhow::Result;
use dotenv::dotenv;
use ethers::types::{
    CallFrame, GethDebugTracerType, GethDebugTracingOptions, GethTrace, GethTraceFrame, H256, U256,
};
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{Signer, Wallet},
    types::{Address, Chain, Transaction, U64},
    utils::{Anvil, AnvilInstance},
};
use futures::TryFutureExt;
use log::{debug, error, info, warn};
use std::sync::Arc;

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
        let wallet = Wallet::from(from_private_key).with_chain_id(Chain::Mainnet);

        // Create the SignerMiddleware
        let client = Arc::new(SignerMiddleware::new(provider, wallet));

        client
            .provider()
            .request::<_, ()>(
                "anvil_setBalance",
                [
                    format!("{:#x}", from_address),
                    "0x56BC75E2D63100000".to_string(), // 100 ETH in wei (hexadecimal)
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
        let swap_router_address: Address = CONTRACT.get_address().uniswap_swap_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        // Create an instance of your specialized Swap Router contract
        let swap_router = UNISWAP_V3_ROUTER::new(swap_router_address, self.client.clone());

        // Impersonate the from_address to send transactions from it
        self.client
            .provider()
            .request::<_, ()>("anvil_impersonateAccount", [self.from_address])
            .await?;

        // Convert 10 ETH to WETH using wrapETH method on the router
        let wrap_amount = ethers::utils::parse_ether("10.0")?;
        let wrap_tx = swap_router.wrap_eth(wrap_amount).value(wrap_amount);

        let pending_wrap = wrap_tx.send().await?;
        let _wrap_receipt = pending_wrap
            .await?
            .ok_or_else(|| anyhow::anyhow!("No wrapETH receipt received"))?;

        // Approve the router to spend WETH (10 WETH)
        let weth_contract = ERC20::new(weth_address, self.client.clone());
        let approval_amount = ethers::utils::parse_ether("10")?;
        weth_contract
            .approve(swap_router_address, approval_amount)
            .send()
            .await?;

        // Stop impersonating the account
        self.client
            .provider()
            .request::<_, ()>("anvil_stopImpersonatingAccount", [self.from_address])
            .await?;

        Ok(())
    }

    async fn simulate_buying_token_for_eth(&self, token: Erc20Token) -> Result<()> {
        let swap_router_address: Address = CONTRACT.get_address().uniswap_swap_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let token_address: Address = token.address.parse()?;
        let swap_router = UNISWAP_V3_ROUTER::new(swap_router_address, self.client.clone());

        // Impersonate the account you want to send the transaction from
        self.client
            .provider()
            .request::<_, ()>("anvil_impersonateAccount", [self.from_address])
            .await?;

        let amount_to_buy =
            std::env::var("TOKEN_TO_BUY_IN_ETH").expect("TOKEN_TO_BUY_IN_ETH is not set in .env");
        let amount_in = ethers::utils::parse_ether(amount_to_buy)?;

        let swap_params = ExactInputSingleParams {
            token_in: weth_address,
            token_out: token_address,
            fee: token.fee,
            recipient: self.from_address,
            amount_in,
            amount_out_minimum: U256::from(0),
            sqrt_price_limit_x96: U256::from(0),
        };

        let tx = swap_router.exact_input_single(swap_params);

        info!("set gas limit for transaction");
        let tx = tx.gas(U256::from(1_000_000));

        // sent transaction
        info!("sending liquidate transcation");
        let pending_tx_result = tx.send().await;

        match pending_tx_result {
            Ok(pending_tx) => {
                // Transaction sent successfully
                info!("Transaction sent, awaiting receipt");
                let tx_hash = pending_tx.tx_hash();
                debug!("tx_hash => {:?}", tx_hash);

                // wait for transaction receipt
                info!("awaiting transaction receipt");
                let receipt = pending_tx.await?.unwrap();
                // info!("transaction receipt obtained ==> {:#?}", receipt);

                let tx_hash = receipt.transaction_hash;

                self.trace_transaction(tx_hash).await?;

                // TODO - get account balance to see how much of new token recieved
                let token_contract = ERC20::new(token_address, self.client.clone());

                let token_balance = token_contract.balance_of(self.from_address).call().await?;

                info!(
                    "YOU HAVE BOUGHT {} of {}, ({})",
                    token_balance, token.name, token.symbol
                );
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
        Ok(())
    }

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
