use super::{
    contracts::CONTRACT,
    token_data::{get_and_save_erc20_by_token_address, update_token},
};
use crate::{
    events::PoolCreatedEvent,
    swap::anvil_simlator::{self, AnvilSimulator},
    utils::type_conversion::address_to_string,
};
use ethers::{
    abi::{token, Address},
    core::types::U256,
    providers::{Middleware, Provider, Ws},
};
use futures::lock::Mutex;
use log::{info, warn};
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub struct Erc20Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub fee: u32,
    pub address: String,
    pub pool_address: String,
    pub is_buyable: bool,
    pub done_buying: bool,
    pub amount_bought: U256,
    pub time_of_purchase: u32,
}

pub async fn add_validate_buy_new_token(
    pool_created_event: &PoolCreatedEvent,
    client: &Arc<Provider<Ws>>,
    anvil: &Arc<AnvilSimulator>,
    timestamp: Arc<Mutex<u32>>,
) -> anyhow::Result<()> {
    let weth_address: Address = CONTRACT.get_address().weth.parse()?;

    // find address of new token
    let token_address = if weth_address == pool_created_event.token0 {
        pool_created_event.token1
    } else if weth_address == pool_created_event.token1 {
        pool_created_event.token0
    } else {
        warn!("not weth pool, skipping");
        return Ok(());
    };

    // TODO - VALIDATE TOKEN HERE - IF SCAM exit out

    let token_address_string = address_to_string(token_address);
    let pool_address_string = address_to_string(pool_created_event.pool);

    // SAVE TOKEN TO GLOBAL STATE
    let token = get_and_save_erc20_by_token_address(
        &token_address_string,
        &pool_address_string,
        pool_created_event.fee,
        client,
    )
    .await?;

    // TEST PURCHASE ON ANVIL FOR NOW
    let token_balance = anvil.simulate_buying_token_for_eth(&token).await?;

    if token_balance > U256::from(0) {
        let time = timestamp.lock().await;
        let updated_token = Erc20Token {
            is_buyable: true,
            amount_bought: token_balance,
            time_of_purchase: *time,
            done_buying: true,
            ..token
        };

        update_token(&updated_token).await;
        info!("token updated and saved");
    }

    Ok(())
}
