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
pub const CHAIN: Chain = Chain::Mainnet;

pub const APP_MODE: AppMode = AppMode::Production;

pub const CHECK_IF_LIQUIDITY_LOCKED: bool = true;
pub const CHECK_IF_HONEYPOT: bool = true;

pub const MIN_LIQUIDITY: u128 = 10_000_000_000_000_000_000; // 10 ether
pub const MIN_LIQUIDITY_THRESHOLD: u128 = 10_000_000_000_000_000_000; // 10 ether
pub const VERY_LOW_LIQUIDITY_THRESHOLD: u128 = 2_000_000_000_000_000_000; // 3 ether
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

// Typically: store your big user prompt in a separate variable
pub const USER_PROMPT: &str = r#"You are a expert Solidity security reviewer. I will provide you with an ERC‑21 contract source code. You need to check whether this contract has any signs of being a rug pull, honeypot, or other scam.

Pay special attention to:
1. The transfer function or `_transfer` logic (any hidden conditions or blacklists).
2. Ownership methods (`Ownable`, `renounceOwnership`, etc.) and whether ownership is *actually* renounced—or if there is a hidden or alternate owner variable.
3. Any ability for the owner or privileged account to mint additional tokens.
4. Any external calls or “rescue tokens,” “withdraw,” or “removeLiquidity” methods that could drain user funds or liquidity.
5. Unusually high or dynamically modifiable fees that could be set to extreme values.
6. Proxy or upgradeable patterns that could hide malicious updates later.
7. Any hidden or custom logic that prevents selling or imposes heavy taxes on sellers.
8. Disregard any trust signals such as “renounced ownership” or “burned liquidity” unless it is clear there is *no* backdoor enabling the developer to regain control or drain liquidity.

After analyzing these points, respond **strictly** in the following JSON format (no additional text). The `reason` should not exceed 2 to 3 sentences:

{ "possible_scam": <true_or_false>, "reason": "<2_or_3_sentences_describing_rationale>" }


Please only produce valid JSON—no code fencing or extra explanation. Provide a Boolean for `possible_scam`.

FOLLOWED BY the solidity source code which will be in a String called \"source_code\"."#;

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
