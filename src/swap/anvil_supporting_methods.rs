use crate::abi::erc20::ERC20;
use crate::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use crate::swap::anvil_simlator::{AnvilSimulator, STARTING_BALANCE};
use crate::utils::type_conversion::{address_to_string, u256_to_f64_with_decimals};
use anyhow::Result;
use ethers::types::{
    CallFrame, GethDebugTracerType, GethDebugTracingOptions, GethTrace, GethTraceFrame, H256, U256,
};
use ethers::utils::format_units;
use ethers::{providers::Middleware, types::Address};
use log::{debug, info};

// ***************** ***************** **************** **********************************
// ***************** SUPPORTING METHODS FOR DEBUGGING AND DIAGNOSIS *****************
// ***************** ***************** **************** **********************************

impl AnvilSimulator {
    pub async fn get_amount_out_uniswap_v2(
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

    pub async fn trace_transaction(&self, tx_hash: H256) -> Result<()> {
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

        // println!("Transaction trace: {:?}", trace);

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

    pub async fn show_weth_allowance_balance_sender_and_pair(
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
        info!("getting token balance");
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

    pub async fn get_weth_balance(&self) -> anyhow::Result<U256> {
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        // get account balance to see how much of new token recieved
        let token_contract = ERC20::new(weth_address, self.client.clone());

        let new_token_balance_u256 = token_contract.balance_of(self.from_address).call().await?;
        let token_balance = format_units(new_token_balance_u256, u32::from(18u32))?;

        println!("YOU HAVE {} of WETH", token_balance);
        Ok(new_token_balance_u256)
    }
}

pub fn find_revert(trace: &CallFrame) -> Option<&CallFrame> {
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
