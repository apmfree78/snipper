use super::simlator::AnvilSimulator;
use crate::abi::erc20::ERC20;
use crate::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use crate::data::contracts::CONTRACT;
use crate::data::tokens::Erc20Token;
use crate::swap::tx_trait::Txs;
use crate::utils::type_conversion::address_to_string;
use anyhow::Result;
use ethers::types::{
    CallFrame, GethDebugTracerType, GethDebugTracingOptions, GethTrace, GethTraceFrame, H256, U256,
};
use ethers::{providers::Middleware, types::Address};
use log::debug;

// ***************** ***************** **************** **********************************
// ***************** SUPPORTING METHODS FOR DEBUGGING AND DIAGNOSIS *****************
// ***************** ***************** **************** **********************************

impl AnvilSimulator {
    pub async fn trace_transaction(&self, tx_hash: H256) -> Result<()> {
        let mut tracing_options = GethDebugTracingOptions::default();
        tracing_options.disable_storage = Some(false); // Enable storage tracing
        tracing_options.disable_stack = Some(false); // Enable stack tracing
        tracing_options.tracer = Some(GethDebugTracerType::BuiltInTracer(
            ethers::types::GethDebugBuiltInTracerType::CallTracer,
        ));

        let trace = self
            .signed_client
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

    pub async fn do_dummy_transfer(
        &self,
        token_address: Address,
        amount: U256,
    ) -> anyhow::Result<()> {
        // 1) Use the ERC20::new(...) constructor
        let token_contract_wallet_1 = ERC20::new(token_address, self.signed_client.clone());
        let token_contract_wallet_2 = ERC20::new(token_address, self.second_signed_client.clone());

        // 2) Transfer from wallet1 -> wallet2
        token_contract_wallet_1
            .transfer(self.second_signed_client.address(), amount)
            .send()
            .await?;

        // 3) Transfer from wallet2 -> wallet1
        token_contract_wallet_2
            .transfer(self.signed_client.address(), amount)
            .send()
            .await?;

        Ok(())
    }

    pub async fn show_eth_uniswap_v2_pair(&self, token: &Erc20Token) -> anyhow::Result<()> {
        let factory_address: Address = CONTRACT.get_address().uniswap_v2_factory.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;

        let factory = UNISWAP_V2_FACTORY::new(factory_address, self.signed_client());

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
    // pub async fn get_weth_balance(&self) -> anyhow::Result<U256> {
    //     let weth_address: Address = CONTRACT.get_address().weth.parse()?;
    //     // get account balance to see how much of new token recieved
    //     let token_contract = ERC20::new(weth_address, self.signed_client.clone());
    //
    //     let new_token_balance_u256 = token_contract.balance_of(self.sender).call().await?;
    //     let token_balance = format_units(new_token_balance_u256, u32::from(18u32))?;
    //
    //     println!("YOU HAVE {} of WETH", token_balance);
    //     Ok(new_token_balance_u256)
    // }
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
