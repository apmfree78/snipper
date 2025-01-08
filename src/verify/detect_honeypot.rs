use anyhow::Result;
use serde::Deserialize;

use crate::data::tokens::Erc20Token;

// ===================
// 1) Structs for JSON
// ===================

#[derive(Debug, Deserialize)]
pub struct HoneypotResponse {
    /// We only care about these two fields from the entire JSON
    pub summary: Summary,
    #[serde(rename = "honeypotResult")]
    pub honeypot_result: HoneypotResult,
}

#[derive(Debug, Deserialize)]
pub struct Summary {
    pub risk: String,
    #[serde(rename = "riskLevel")]
    pub risk_level: u64,
    /// The endpoint can also return an array of flags if present:
    #[serde(default)]
    pub flags: Vec<SummaryFlag>,
}

#[derive(Debug, Deserialize)]
pub struct SummaryFlag {
    pub flag: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(rename = "severityIndex")]
    pub severity_index: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct HoneypotResult {
    #[serde(rename = "isHoneypot")]
    pub is_honeypot: bool,
    /// This field might not exist if `isHoneypot == false`, so we use Option
    #[serde(rename = "honeypotReason")]
    pub honeypot_reason: Option<String>,
}

// ===================
// 2) The function
// ===================

/// Calls the Honeypot.is API to check if a given address is a honeypot
///
/// # Arguments
/// - `token_or_pair_address`: The contract address to test (e.g. the pair address)
///
/// # Returns
/// - (Summary, HoneypotResult) on success
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let address = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"; // USDC
///     let (summary, honeypot_result) = is_honeypot(address).await?;
///
///     println!("Risk: {}", summary.risk);
///     println!("Is honeypot? {}", honeypot_result.isHoneypot);
///     if let Some(reason) = honeypot_result.honeypotReason {
///         println!("Reason: {}", reason);
///     }
///     Ok(())
/// }
/// ```
///
impl Erc20Token {
    pub async fn is_honeypot(&self) -> Result<(Summary, HoneypotResult)> {
        // 2) Build the request
        let client = reqwest::Client::new();

        // The endpoint: https://api.honeypot.is/v2/IsHoneypot
        // Query param:  address=<YOUR_ADDRESS>
        // Header:       X-API-KEY: <YOUR_APIKEY>
        let url = "https://api.honeypot.is/v2/IsHoneypot";

        // 4) Make GET request with `address` param
        let resp = client
            .get(url)
            .query(&[("address", self.pair_address)])
            .send()
            .await?
            .error_for_status()?; // If not 200..299, produce an error

        // 5) Deserialize JSON into our HoneypotResponse
        let parsed: HoneypotResponse = resp.json().await?;

        // 6) Return the fields we care about
        Ok((parsed.summary, parsed.honeypot_result))
    }
}
