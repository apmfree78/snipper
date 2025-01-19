use anyhow::{anyhow, Result};
use ethers::abi::{Address, Bytes};
use ethers::utils::{format_units, keccak256};
use ethers::{
    core::types::{transaction::eip2718::TypedTransaction, H256, I256, U256},
    types::{Eip1559TransactionRequest, NameOrAddress, Transaction, TransactionRequest},
};
use rust_decimal::Decimal;
use std::convert::TryInto;
use std::str::FromStr;

use crate::app_config::TIME_ROUNDS;

pub fn get_function_selector(function_signature: &str) -> Bytes {
    let hash = H256::from(keccak256(function_signature.as_bytes()));
    Bytes::from(&hash[0..4])
}

pub fn format_to_5_decimals_decimal(amount: U256, decimals: u32) -> String {
    let amount_str = format_units(amount, decimals).unwrap_or_default();
    // Convert that string to a Decimal
    let decimal_val = Decimal::from_str(&amount_str).unwrap_or_default();
    // Round to 5 decimal places
    let truncated_val = decimal_val.round_dp(5);
    // Convert back to a string
    truncated_val.to_string()
}

pub fn convert_transaction_to_typed_transaction(tx: &Transaction) -> TypedTransaction {
    let to = tx.to.clone().unwrap();
    let to = NameOrAddress::Address(to);

    if tx.transaction_type == Some(2.into()) {
        // EIP-1559 transaction
        TypedTransaction::Eip1559(Eip1559TransactionRequest {
            from: Some(tx.from),
            to: Some(to),
            value: Some(tx.value),
            data: Some(tx.input.clone()),
            nonce: Some(tx.nonce),
            gas: Some(tx.gas),
            max_fee_per_gas: tx.max_fee_per_gas,
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas,
            access_list: tx.access_list.clone().unwrap_or_default(),
            chain_id: tx.chain_id.map(|id| ethers::types::U64::from(id.as_u64())),
        })
    } else {
        // Legacy transaction
        TypedTransaction::Legacy(TransactionRequest {
            from: Some(tx.from),
            to: Some(to),
            value: Some(tx.value),
            data: Some(tx.input.clone()),
            nonce: Some(tx.nonce),
            gas: Some(tx.gas),
            gas_price: tx.gas_price,
            chain_id: tx.chain_id.map(|id| ethers::types::U64::from(id.as_u64())),
        })
    }
}

pub fn get_time_interval(
    current_time: u32,
    time_of_purchase: u32,
) -> anyhow::Result<Option<usize>> {
    let time_interval =
        std::env::var("TOKEN_SELL_INTERVAL").expect("TOKEN_SELL_INTERVAL not found in .env");
    let time_interval: u32 = time_interval.parse()?;
    let diff = current_time as usize - time_of_purchase as usize;

    // find time index to make purchase on
    let time_index = diff / time_interval as usize;

    // check that time index is in range for what time slots we want to test within
    if time_index < TIME_ROUNDS {
        Ok(Some(time_index))
    } else {
        Ok(None)
    }
}

pub fn u256_to_f64_with_decimals(value: U256, decimals: u32) -> anyhow::Result<f64> {
    // Convert U256 to a string with the given decimals
    let value_str = format_units(value, decimals)?;
    // Parse the decimal string into a f64
    let value_f64 = f64::from_str(&value_str)?;
    Ok(value_f64)
}

pub fn str_to_h256_hash(str: &str) -> H256 {
    H256::from(keccak256(str.as_bytes()))
}

pub fn u256_to_bytes_array(number: U256) -> [u8; 32] {
    let mut number_bytes = [0u8; 32];
    number.to_big_endian(&mut number_bytes);
    number_bytes
}

pub fn boolean_to_bytes_array(boolean: bool) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[31] = if boolean { 1 } else { 0 };
    bytes
}

pub fn u8_to_bytes_array(value: u8) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[31] = value;
    bytes
}

pub fn u16_to_bytes_array(value: u16) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[30..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

pub fn address_to_bytes_array(address: Address) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[12..32].copy_from_slice(address.as_bytes());
    bytes
}

pub fn address_to_string(address: Address) -> String {
    format!("{:?}", address)
}

pub fn string_to_bool(s: &str) -> anyhow::Result<bool> {
    match s.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(anyhow!("Invalid boolean string")),
    }
}

pub fn f64_to_u256(value: f64) -> Result<U256> {
    if value.is_nan() || value.is_infinite() {
        return Err(anyhow!("Invalid f64 value: NaN or Infinity"));
    }

    if value < 0.0 {
        return Err(anyhow!("Negative values are not supported"));
    }

    // Scale the float by 1e18 to move the decimal point
    let scaled_value = value * 1e18;

    // Check for overflow/underflow issues before conversion
    if scaled_value > u128::MAX as f64 {
        return Err(anyhow!("Value is too large for u128"));
    }

    // Safely convert the scaled float to u128, assuming we are within range now
    let integer_value = scaled_value as u128;

    // Convert u128 to U256
    Ok(U256::from(integer_value))
}

pub fn u257_to_f64(value: U256) -> Option<f64> {
    // Convert U256 to u128 safely, if possible (U256 might be larger than u128 can handle)
    if let Ok(val) = value.try_into() {
        let val_u128: u128 = val;
        // Now convert u128 to f64; this conversion is generally safe as f64 can represent u128 values
        // in its range, but precision might be lost for very large u128 values.
        Some(val_u128 as f64)
    } else {
        // The U256 value was too large to fit into a u128
        None
    }
}

pub fn i256_to_f64(value: I256) -> Result<f64, &'static str> {
    // Check if the value fits within f64 precision limits
    if value < I256::from(2_i128.pow(53)) && value > I256::from(-2_i128.pow(53)) {
        // Safe to convert to i64 and then to f64
        let value_i64 =
            i64::try_from(value).map_err(|_| "Conversion to i64 failed due to overflow")?;
        Ok(value_i64 as f64)
    } else {
        Err("Value out of f64 precision range")
    }
}

pub fn h256_to_address(h: &H256) -> Address {
    let bytes = h.as_bytes();
    // Addresses are the last 20 bytes of the H256
    Address::from_slice(&bytes[12..32])
}

pub fn h256_to_u256(h: &H256) -> U256 {
    U256::from_big_endian(h.as_bytes())
}
