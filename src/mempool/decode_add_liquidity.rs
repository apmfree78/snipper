use anyhow::{anyhow, Result};
use ethers::{
    abi::{Abi, Token},
    types::Address,
};

pub fn decode_add_liquidity_eth_fn(data: &Vec<u8>) -> Result<Address> {
    let decode_json = r#"
        [{
            "inputs": [
                {"type": "address", "name": "token"},
                {"type": "uint", "name": "amountTokenDesired"}
                {"type": "uint", "name": "amountTokenMin"}
                {"type": "uint", "name": "amountETHMin"}
                {"type": "address", "name": "to"},
                {"type": "uint", "name": "deadline"}
            ],
            "name": "addLiquidityETH",
            "outputs": [],
            "type": "function"
        }]
    "#;

    let abi = Abi::load(decode_json.as_bytes())?;

    if let Ok(decode_fn) = abi.function("addLiquidityETH") {
        if let Ok(decoded) = decode_fn.decode_input(&data[4..]) {
            match decoded.as_slice() {
                [Token::Address(token), Token::Uint(_), Token::Uint(_), Token::Uint(_), Token::Address(_)] => {
                    Ok(*token)
                }
                _ => Err(anyhow!("Failed to match decoded data with expected types")),
            }
        } else {
            Err(anyhow!("Failed to decode forward data"))
        }
    } else {
        Err(anyhow!("forward function not found in ABI"))
    }
}
