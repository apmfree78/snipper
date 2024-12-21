use crate::data::token_data::update_token_gas_cost;
use crate::data::tokens::Erc20Token;

use ethers::types::{TransactionReceipt, U256};
use ethers::utils::format_units;
use log::{error, warn};

pub async fn update_tx_gas_cost_data(
    receipt: &TransactionReceipt,
    token: &Erc20Token,
) -> anyhow::Result<()> {
    let gas_cost_option = get_tx_gas_cost(&receipt)?;
    match gas_cost_option {
        Some(gas_cost) => {
            update_token_gas_cost(token.address, gas_cost).await;
        }
        None => error!("error calculating gas cost of tx"),
    };
    Ok(())
}

pub fn get_tx_gas_cost(receipt: &TransactionReceipt) -> anyhow::Result<Option<U256>> {
    // Get gas used and gas price (for EIP-1559 transactions, effective_gas_price is used)
    if let (Some(gas_used), Some(gas_price)) = (receipt.gas_used, receipt.effective_gas_price) {
        // Convert gas cost from wei to ether: 1 Ether = 1e18 wei
        let gas_cost_in_wei = gas_used * gas_price;
        let gas_cost_ether = format_units(gas_cost_in_wei, "ether")?;

        println!("Gas cost for the transaction: {} ETH", gas_cost_ether);
        Ok(Some(gas_cost_in_wei))
    } else {
        warn!("No gas usage or gas price info in receipt");
        Ok(None)
    }
}
