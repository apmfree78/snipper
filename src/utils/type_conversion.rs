use anyhow::{anyhow, Result};
use ethers::abi::Address;
use ethers::core::types::U256;
use ethers::types::{H256, I256};
use ethers::utils::keccak256;
use std::convert::TryInto;

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

pub fn f64_to_u256(value: f64) -> Result<U256> {
    if value.is_nan() || value.is_infinite() {
        return Err(anyhow!("Invalid f64 value: NaN or Infinity"));
    }

    // Scale the float by 1e18 to move the decimal point
    let scaled_value = value * 1e18;

    // Convert the scaled float to u64, handling potential overflow
    let integer_value = u64::try_from(scaled_value.round() as i64)
        .map_err(|_| anyhow!("Overflow in conversion to u64"))?;

    // Convert u64 to U256
    Ok(U256::from(integer_value))
}

pub fn u256_to_f64(value: U256) -> Option<f64> {
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
