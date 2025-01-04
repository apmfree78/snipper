use anyhow::anyhow;
use ethers::{
    abi::{Address, RawLog},
    core::types::Filter,
    types::{Log, TransactionReceipt, H256, U256},
    utils::keccak256,
};

use crate::data::{contracts::CONTRACT, tokens::Erc20Token};

// PairCreated (index_topic_1 address token0, index_topic_2 address token1, address pair, uint256 noname)
pub const PAIR_CREATED_SIGNATURE: &str = "PairCreated(address,address,address,uint256)";
pub const SWAP_SIGNATURE: &str = "Swap(address,uint256,uint256,uint256,uint256,address)";
// pub const POOL_CREATED_SIGNATURE: &str = "PoolCreated(address,address,uint24,int24,address)";

#[derive(Debug, Clone)]
pub struct PoolCreatedEvent {
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub pool: Address,
}

#[derive(Debug, Clone)]
pub struct PairCreatedEvent {
    pub token0: Address,
    pub token1: Address,
    pub pair: Address,
    pub noname: U256,
}

#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub sender: Address,
    pub to: Address,
    pub amount_in_0: U256,
    pub amount_in_1: U256,
    pub amount_out_0: U256,
    pub amount_out_1: U256,
}

pub fn set_signature_filter() -> anyhow::Result<Filter> {
    let factory_address = CONTRACT.get_address().uniswap_v2_factory.clone();

    let filter = Filter::new()
        .address(factory_address.parse::<Address>()?)
        .events([PAIR_CREATED_SIGNATURE].to_vec());
    Ok(filter)
}

pub fn decode_swap_event(log: &Log) -> anyhow::Result<SwapEvent> {
    let sender: Address = log.topics[1].into();
    let to: Address = log.topics[2].into();

    // Assuming the data contains the rest in order: user, amount, interestRateMode, borrowRate
    // Proceed with decoding data which is just raw binary (not RLP encoded)
    let raw_log: RawLog = RawLog::from(log.clone());
    let data_slice = raw_log.data;
    if data_slice.len() < 128 {
        return Err(anyhow!("Data field too short to decode all fields"));
    }

    let amount_in_0 = U256::from_big_endian(&data_slice[..32]);
    let amount_in_1 = U256::from_big_endian(&data_slice[32..64]);
    let amount_out_0 = U256::from_big_endian(&data_slice[64..96]);
    let amount_out_1 = U256::from_big_endian(&data_slice[96..128]);

    let swap_event = SwapEvent {
        sender,
        to,
        amount_in_0,
        amount_in_1,
        amount_out_0,
        amount_out_1,
    };

    Ok(swap_event)
}

pub fn decode_pair_created_event(log: &Log) -> anyhow::Result<PairCreatedEvent> {
    let token0: Address = log.topics[1].into();
    let token1: Address = log.topics[2].into();

    // Assuming the data contains the rest in order: user, amount, interestRateMode, borrowRate
    // Proceed with decoding data which is just raw binary (not RLP encoded)
    let raw_log: RawLog = RawLog::from(log.clone());
    let data_slice = raw_log.data;
    if data_slice.len() < 64 {
        return Err(anyhow!("Data field too short to decode all fields"));
    }

    let pair = Address::from_slice(&data_slice[12..32]);
    let noname = U256::from_big_endian(&data_slice[32..64]);

    let pair_created_event = PairCreatedEvent {
        token0,
        token1,
        pair,
        noname,
    };

    Ok(pair_created_event)
}

pub fn decode_pool_created_event(log: &Log) -> anyhow::Result<PoolCreatedEvent> {
    let token0: Address = log.topics[1].into();
    let token1: Address = log.topics[2].into();
    // fee is indexed and stored in topics[3]
    let fee_u256 = U256::from_big_endian(log.topics[3].as_bytes());
    // Extract lower 24 bits for fee
    let fee = (fee_u256.low_u32() & 0xFFFFFF) as u32;

    // Assuming the data contains the rest in order: user, amount, interestRateMode, borrowRate
    // Proceed with decoding data which is just raw binary (not RLP encoded)
    let raw_log: RawLog = RawLog::from(log.clone());
    let data_slice = raw_log.data;
    if data_slice.len() < 64 {
        return Err(anyhow!("Data field too short to decode all fields"));
    }

    // tickSpacing (int24) is in the first 32 bytes of data
    let tick_spacing_bytes = &data_slice[0..32];
    let tick_spacing_u256 = U256::from_big_endian(tick_spacing_bytes);
    let tick_spacing_raw = tick_spacing_u256.low_u32() & 0xFFFFFF;

    // Sign-extend for int24
    let tick_spacing = if (tick_spacing_raw & 0x800000) != 0 {
        (tick_spacing_raw as i32) - 0x1000000
    } else {
        tick_spacing_raw as i32
    };

    let pool = Address::from_slice(&data_slice[44..64]);

    let poolcreated_event = PoolCreatedEvent {
        token0,
        token1,
        fee,
        tick_spacing,
        pool,
    };

    Ok(poolcreated_event)
}
pub fn parse_swap_receipt_logs_to_get_eth_amount_out(
    receipt: &TransactionReceipt,
    token: &Erc20Token,
) -> anyhow::Result<U256> {
    let swap_signature = get_signature_hash(SWAP_SIGNATURE);

    for log in &receipt.logs {
        if log.topics.get(0) == Some(&swap_signature) {
            let swap_event = decode_swap_event(log)?;

            if token.is_token_0 {
                return Ok(swap_event.amount_out_1);
            } else {
                return Ok(swap_event.amount_out_0);
            }
        }
    }
    //  if no matching log found
    Ok(U256::zero())
}

pub fn get_signature_hash(interface: &str) -> H256 {
    H256::from(keccak256(interface.as_bytes()))
}
