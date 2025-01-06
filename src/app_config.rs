use ethers::types::Chain;

#[derive(Debug, PartialEq, Eq)]
pub enum AppMode {
    Production,
    Simulation,
}

//*****************************************
//*****************************************
//*****************************************
//*****************************************
//*****************************************
// CHANGE THIS VALUE TO SET CHAIN AND MODE FOR APP
pub const CHAIN: Chain = Chain::Base;

pub const APP_MODE: AppMode = AppMode::Simulation;

pub const MIN_LIQUIDITY: u128 = 10_000_000_000_000_000_000; // 10 ether
pub const MIN_LIQUIDITY_THRESHOLD: u128 = 10_000_000_000_000_000_000; // 10 ether
pub const VERY_LOW_LIQUIDITY_THRESHOLD: u128 = 1_000_000_000_000_000_000; // 10 ether
pub const LOW_LIQUIDITY_THRESHOLD: u128 = 5_000_000_000_000_000_000; // 10 ether
pub const MEDIUM_LIQUIDITY_THRESHOLD: u128 = 10_000_000_000_000_000_000; // 10 ether
pub const HIGH_LIQUIDITY_THRESHOLD: u128 = 20_000_000_000_000_000_000; // 10 ether

pub const MIN_TRADE_FACTOR: u64 = 10;
pub const MIN_RESERVE_ETH_FACTOR: u64 = 10;

pub const TIME_ROUNDS: usize = 7;
//*****************************************
//*****************************************
//*****************************************
//*****************************************
//*****************************************
