use ethers::types::Chain;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashMap, fs};

use crate::app_config::CHAIN;

pub const ETH: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";
pub const BTC: &str = "0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB";
pub const USD: &str = "0x0000000000000000000000000000000000000348";

pub struct ContractAddresses {
    pub weth: String,
    pub link: String,
    pub uniswap_swap_router: String,
    pub uniswap_v2_router: String,
    pub uniswap_v2_factory: String,
    pub uniswap_factory: String,
    pub uniswap_quoter: String,
    pub ws_url: String,
    pub http_url: String,
    pub alchemy_url: String,
}

pub struct ContractAddressMap {
    pub addresses: HashMap<Chain, ContractAddresses>,
}

impl ContractAddressMap {
    fn new(chains: Chains) -> Self {
        let mut addresses = HashMap::<Chain, ContractAddresses>::new();
        addresses.insert(
            Chain::Base,
            ContractAddresses {
                uniswap_factory: chains.base.uniswap_factory,
                uniswap_swap_router: chains.base.uniswap_swap_router,
                uniswap_v2_factory: chains.base.uniswap_v2_factory,
                uniswap_v2_router: chains.base.uniswap_v2_router,
                uniswap_quoter: chains.base.uniswap_quoter,
                weth: chains.base.weth,
                link: chains.base.link,
                ws_url: chains.base.ws_url,
                http_url: chains.base.http_url,
                alchemy_url: chains.base.alchemy_url,
            },
        );
        addresses.insert(
            Chain::Mainnet,
            ContractAddresses {
                uniswap_factory: chains.mainnet.uniswap_factory,
                uniswap_swap_router: chains.mainnet.uniswap_swap_router,
                uniswap_quoter: chains.mainnet.uniswap_quoter,
                uniswap_v2_factory: chains.mainnet.uniswap_v2_factory,
                uniswap_v2_router: chains.mainnet.uniswap_v2_router,
                weth: chains.mainnet.weth,
                link: chains.mainnet.link,
                ws_url: chains.mainnet.ws_url,
                http_url: chains.mainnet.http_url,
                alchemy_url: chains.mainnet.alchemy_url,
            },
        );
        Self { addresses }
    }

    pub fn get_address(&self) -> &ContractAddresses {
        self.addresses.get(&CHAIN).expect("Chain not supported")
    }
}

#[derive(Deserialize)]
struct Chains {
    base: ChainContracts,
    mainnet: ChainContracts,
}

#[derive(Deserialize)]
struct ChainContracts {
    uniswap_swap_router: String,
    uniswap_factory: String,
    uniswap_quoter: String,
    uniswap_v2_router: String,
    uniswap_v2_factory: String,
    weth: String,
    link: String,
    ws_url: String,
    http_url: String,
    alchemy_url: String,
}

impl Chains {
    fn load() -> Self {
        let config = fs::read_to_string("contracts.toml").expect("failed to read toml file");
        toml::from_str(&config).expect("failed to parse toml to chain config")
    }
}

// CREATE GLOBAL INSTANCE OF ALL CONTRACT ADDRESS FOR A GIVEN CHAIN
pub static CONTRACT: Lazy<ContractAddressMap> = Lazy::new(|| {
    let chains = Chains::load();

    ContractAddressMap::new(chains)
});
