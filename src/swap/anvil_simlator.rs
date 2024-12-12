use crate::abi::erc20::ERC20;
use crate::abi::uniswap_factory_v2::UNISWAP_V2_FACTORY;
use crate::abi::uniswap_router_v2::UNISWAP_V2_ROUTER;
use crate::data::contracts::{CHAIN, CONTRACT};
use crate::data::tokens::Erc20Token;
use crate::utils::type_conversion::{
    address_to_string, get_function_selector, u256_to_f64_with_decimals,
};
use anyhow::Result;
use ethers::types::{
    CallFrame, GethDebugTracerType, GethDebugTracingOptions, GethTrace, GethTraceFrame,
    TransactionRequest, H256, U256,
};
use ethers::utils::format_units;
use ethers::{
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Middleware, Provider, Ws},
    signers::{Signer, Wallet},
    types::Address,
    utils::{Anvil, AnvilInstance},
};
use log::{debug, error, info};
use std::sync::Arc;

pub const STARTING_BALANCE: f64 = 1000.0;

#[derive(Debug, Clone, Copy)]
pub enum UniswapVersion {
    V2,
    V3,
}

pub struct AnvilSimulator {
    pub client: Arc<SignerMiddleware<Provider<Ws>, Wallet<SigningKey>>>,
    pub anvil: AnvilInstance,
    pub from_address: Address,
}

impl AnvilSimulator {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        // Main network provider   // Configure Anvil with forking
        let anvil = Anvil::new()
            .fork(rpc_url) // URL of your Geth node
            .spawn();

        // setup mock sender
        let from_address: Address = anvil.addresses()[0];
        let private_keys = anvil.keys();
        let from_private_key = private_keys[0].clone();

        // Connect to Anvil
        let anvil_ws_url = anvil.ws_endpoint();
        let provider = Provider::<Ws>::connect(anvil_ws_url).await?;

        // Create a wallet with the private key
        let wallet = Wallet::from(from_private_key).with_chain_id(CHAIN);

        // Create the SignerMiddleware
        let client = Arc::new(SignerMiddleware::new(provider, wallet));

        client
            .provider()
            .request::<_, ()>(
                "anvil_setBalance",
                [
                    format!("{:#x}", from_address),
                    "0x3635c9adc5dea00000".to_string(), //100 ETH
                ],
            )
            .await?;

        let simulator = Self {
            client,
            anvil,
            from_address,
        };

        simulator.prepare_account().await?;

        Ok(simulator)
    }

    /// Prepares the test account by converting some ETH to WETH and approving the router.
    pub async fn prepare_account(&self) -> anyhow::Result<()> {
        let router_address: Address = CONTRACT.get_address().uniswap_v2_router.parse()?;
        let weth_address: Address = CONTRACT.get_address().weth.parse()?;
        let weth_contract = ERC20::new(weth_address, self.client.clone());

        // Impersonate the from_address to send transactions from it
        self.client
            .provider()
            .request::<_, ()>("anvil_impersonateAccount", [self.from_address])
            .await?;

        // Convert 10 ETH to WETH using wrapETH method on the router
        let wrap_amount = ethers::utils::parse_ether("30.0")?;

        let eth_balance = self.client.get_balance(self.from_address, None).await?;
        debug!("ETH Balance of from_address: {}", eth_balance);
        debug!("amount of eth to wrap => {}", wrap_amount);

        if eth_balance < wrap_amount {
            return Err(anyhow::anyhow!(
                "Insufficient ETH balance: required {}, available {}",
                wrap_amount,
                eth_balance
            ));
        }

        let deposit_selector = get_function_selector("deposit()");

        let gas_price = self.client.provider().get_gas_price().await?;
        debug!("Current gas price: {}", gas_price);

        // Wrap 10 ETH into WETH
        let wrap_tx = self
            .client
            .provider()
            .send_transaction(
                TransactionRequest::new()
                    .to(weth_address)
                    .data(deposit_selector)
                    .value(wrap_amount)
                    .gas_price(gas_price)
                    .gas(U256::from(300_000)),
                None,
            )
            .await?;

        let _wrap_receipt = wrap_tx
            .await?
            .ok_or_else(|| anyhow::anyhow!("No wrapETH receipt received"))?;

        let approval_amount = ethers::utils::parse_ether("30")?;
        weth_contract
            .approve(router_address, approval_amount)
            .send()
            .await?;

        let weth_balance = weth_contract.balance_of(self.from_address).call().await?;
        let allowance = weth_contract
            .allowance(self.from_address, router_address)
            .call()
            .await?;
        debug!("WETH Balance of anvil mock account: {}", weth_balance);
        debug!("Allowance of anvil mock account:  {}", allowance);

        // Stop impersonating the account
        self.client
            .provider()
            .request::<_, ()>("anvil_stopImpersonatingAccount", [self.from_address])
            .await?;

        Ok(())
    }
}
