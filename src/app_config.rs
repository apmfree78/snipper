use ethers::types::Chain;

use crate::verify::ai_submission::AIModel;

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

pub const AI_MODEL: AIModel = AIModel::OpenAi;

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
pub const TOKEN_HOLDER_THRESHOLD_PERCENTAGE: u64 = 10;
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
pub const WEBSITE_MAX_CHARACTER_LENGTH: u32 = 40_000;

pub const PURCHASE_ATTEMPT_LIMIT: u8 = 5;
pub const SELL_ATTEMPT_LIMIT: u8 = 10;

pub const API_CHECK_LIMIT: u8 = 10;

pub const BLACKLIST: [&str; 1] = ["CHILLI"];

pub const FINAL_DETERMINATION_PROMPT: &str = r#"You are a senior crypto investigator. I will provide two JSON assessments:

1. The result of analyzing an ERC-20 contract’s source code for potential scams or malicious features.
2. The result of analyzing the project's website for credibility.

Each assessment will have a `"possible_scam"` boolean and a `"reason"` field, plus additional summary/context if available. Using these two inputs, please:

1. Combine the findings: If the website is credible but the code is suspicious (or vice versa), decide whether the suspicious code might be legitimately used to defend against malicious trading bots, front-running, sniping, etc.
2. Make a final determination: Is this token possibly a scam or not?

Your **output must be strictly valid JSON** (no extra text or code fencing), in this format:

{
  "final_scam_assessment": <true_or_false>,
  "reason": "<2-3_sentences_explaining_why_you_concluded_scam_or_not>",
  "could_legitimately_justify_suspicious_code": <true_or_false>
}

Where:
- `final_scam_assessment` is a Boolean indicating if you think the token is likely a scam overall.
- `reason` is a concise (2–3 sentences) explanation.
- `could_legitimately_justify_suspicious_code` is a Boolean indicating whether the suspicious code could be explained by legitimate anti-bot or anti-exploit measures.
"#;
pub const CODE_CHECK_PROMPT: &str = r#"You are an expert Solidity security reviewer. I will provide you with an ERC-20 contract source code. You need to check whether this contract has any signs of being a rug pull, honeypot, or other scam.

Pay special attention to:
1. The transfer function or `_transfer` logic (any hidden conditions or blacklists).
2. Ownership methods (`Ownable`, `renounceOwnership`, etc.) and whether ownership is *actually* renounced—or if there is a hidden or alternate owner variable.
3. Any ability for the owner or privileged account to mint additional tokens.
4. Any external calls or “rescue tokens,” “withdraw,” or “removeLiquidity” methods that could drain user funds or liquidity.
5. Unusually high or dynamically modifiable fees that could be set to extreme values.
6. Proxy or upgradeable patterns that could hide malicious updates later.
7. Any hidden or custom logic that prevents selling or imposes heavy taxes on sellers.
8. Disregard any trust signals such as "renounced ownership" or "burned liquidity" unless it is clear there is *no* backdoor enabling the developer to regain control or drain liquidity.

After analyzing these points, respond **strictly** in the following JSON format (no additional text). The `reason` should not exceed 2 to 3 sentences:

{ 
  "possible_scam": <true_or_false>, 
  "reason": "<2_or_3_sentences_describing_rationale>" 
  "could_legitimately_justify_suspicious_code": <true_or_false>
  "reason_could_be_legitimate_or_not": <2_or_3_sentences_describing_rationale>
}

*Note*: 

- `could_legitimately_justify_suspicious_code` is a Boolean indicating whether the suspicious code could reasonably be explained by legitimate anti-bot, anti-snipper, or anti-exploit measures,
 that could be implimented by legitimate smart contracts. . If there is no suspicious code just set to true.
`reason_could_be_legitimate_or_not` - If you set `could_legitimately_justify_suspicious_code` to true, THEN explain in a couple sentences WHY any suspicious code in contract could be legitimate anti-bot/anti-snipper/anti-exploit measures, 
 however if you set `could_legitimately_justify_suspicious_code` to false explain WHY. If there is no suspicious code just state "code is legitimate".

Please only produce valid JSON—no code fencing or extra explanation. Provide a Boolean for `possible_scam`.

FOLLOWED BY the solidity source code which will be in a String called \"source_code\".

Return **only** valid JSON. Do NOT include triple backticks or any other formatting around the JSON.
"#;

pub const WEBSITE_CHECK_PROMPT: &str = r#"You are an expert crypto investigator specializing in website credibility. I will provide the text scraped from a token project's website. Please:

1. Determine if the website seems credible or if it exhibits potential scam signals.  
2. Focus on any obvious contradictions, fake partnerships, poorly written or copy-pasted content, unrealistic claims, or missing critical information (e.g., team info, roadmap, contact details).  
3. Summarize the site’s content (in your own words) and highlight any suspicious indicators.

**Output must be strictly valid JSON** with this structure (no extra text or code fencing):

{
  "possible_scam": <true_or_false>,
  "reason": "<2_or_3_sentences_here>",
  "summary": "<overview_of_website_and_why_it’s_credible_or_not>"
}

Where:
- `possible_scam` is a Boolean that indicates your judgment on whether the website is likely credible or possibly a scam.
- `reason` is a concise (2–3 sentence) explanation of why you concluded so.
- `summary` is a short overview (under 2000 tokens for the entire JSON response) describing key points of the website and its credibility.

Return only valid JSON. Do NOT include triple backticks or any other formatting around the JSON."#;

pub const TWITTER_PROMPT: &str = r#"You are an expert crypto investigator specializing in social media analysis. I will provide the Twitter profile’s stats of a crypto project (e.g., follower count, following count, account age) and its 50 most recent posts. Please:

1. Determine if the account seems credible or if it exhibits potential scam signals.
2. Look for inconsistencies, copy-pasted or repetitive “shilling,” unrealistic claims, suspicious engagement patterns, or evidence of bot activity. Also note any red flags from extremely low engagement relative to follower count, newly created accounts with huge follower spikes, or significant mismatches between followers and actual interactions.
3. Summarize the account’s overall behavior (in your own words) and highlight any red flags that might indicate a scam. This summary should be no longer than 500 tokens.
**Output must be strictly valid JSON** with this structure (no extra text or code fencing):

{
  "possible_scam": <true_or_false>,
  "reason": "<2_or_3_sentences_here>",
  "summary": "<brief_overview_of_the_tweets_account_stats_and_why_it’s_credible_or_not>"
}

Where:
- `possible_scam` is a Boolean that indicates your judgment on whether the Twitter account is likely credible or possibly a scam.
- `reason` is a concise (2–3 sentence) explanation of why you concluded so.
- `summary` is a short overview describing key points of the account’s content, engagement levels, follower stats, post history, etc. (limit the entire JSON response to under 2000 tokens).
"#;

//*****************************************
//*****************************************
//*****************************************
//*****************************************
//*****************************************
