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

pub const APP_MODE: AppMode = AppMode::Production;

pub const CHECK_IF_LIQUIDITY_LOCKED: bool = true;
pub const CHECK_IF_HONEYPOT: bool = true;

pub const MIN_LIQUIDITY: u128 = 10_000_000_000_000_000_000; // 10 ether
pub const MIN_LIQUIDITY_THRESHOLD: u128 = 10_000_000_000_000_000_000; // 10 ether
pub const VERY_LOW_LIQUIDITY_THRESHOLD: u128 = 2_000_000_000_000_000_000; // 1 ether
pub const LOW_LIQUIDITY_THRESHOLD: u128 = 10_000_000_000_000_000_000; // 5 ether
pub const MEDIUM_LIQUIDITY_THRESHOLD: u128 = 15_000_000_000_000_000_000; // 10 ether
pub const HIGH_LIQUIDITY_THRESHOLD: u128 = 20_000_000_000_000_000_000; // 20 ether

pub const MIN_TRADE_FACTOR: u64 = 10;
pub const MIN_RESERVE_ETH_FACTOR: u64 = 10;

pub const TIME_ROUNDS: usize = 10;

pub const LIQUIDITY_PERCENTAGE_LOCKED: u64 = 90;
pub const TOKEN_LOCKERS_MAINNET: [&str; 4] = [
    "0xe2fe530c047f2d85298b07d9333c05737f1435fb", // team finance (lowercased)
    "0x663a5c229c09b049e36dcc11a9b0d4a8eb9db214", // UNCX (lowercased)
    "0x000000000000000000000000000000000000dead", // token burn (lowercased)
    "0x0000000000000000000000000000000000000000", // token burn
];
pub const TOKEN_LOCKERS_BASE: [&str; 3] = [
    "0xc4e637d37113192f4f1f060daebd7758de7f4131", // UNCX (lowercased)
    "0x000000000000000000000000000000000000dead", // token burn (lowercased)
    "0x0000000000000000000000000000000000000000", // token burn
];

pub const CONTRACT_TOKEN_SIZE_LIMIT: u32 = 15_000;

pub const PURCHASE_ATTEMPT_LIMIT: u8 = 5;
pub const SELL_ATTEMPT_LIMIT: u8 = 10;

pub const API_CHECK_LIMIT: u8 = 10;

pub const BLACKLIST: [&str; 1] = ["CHILLI"];

//  const TOKEN_LOCKERS: [&str; 4] = [
//     "0xE2fE530C047f2d85298b07D9333C05737f1435fB", // team finance
//     "0x663A5C229c09b049E36dCc11a9B0d4a8Eb9db214", // UNCX
//     "0x000000000000000000000000000000000000dEaD", // token burn
//     "0x0000000000000000000000000000000000000000", // token burn
// ];
//*****************************************
//*****************************************
//*****************************************
//*****************************************
//*****************************************
