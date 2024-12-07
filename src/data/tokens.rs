use ethers::core::types::U256;

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
}
