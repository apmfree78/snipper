pub mod abi {
    pub mod erc20;
    pub mod uniswap_factory_v2;
    pub mod uniswap_pair;
    pub mod uniswap_pool;
    pub mod uniswap_quoter;
    pub mod uniswap_router_v2;
    pub mod uniswap_v3_factory;
    pub mod uniswap_v3_router;
}

pub mod mempool {
    pub mod decode_add_liquidity;
    pub mod detect_add_liquidity;
}

pub mod utils {
    pub mod logging;
    pub mod tx;
    pub mod type_conversion;
}

pub mod data {
    pub mod contracts;
    pub mod gas;
    pub mod portfolio;
    pub mod token_data;
    pub mod tokens;
}

pub mod token_tx {
    pub mod anvil;
    pub mod time_intervals;
    pub mod tx;
    pub mod validate;
    pub mod volume_intervals;
}

pub mod app_config;
pub mod events;

pub mod swap {
    pub mod anvil {
        pub mod buy_sell;
        pub mod simlator;
        pub mod snapshots;
        pub mod supporting_methods;
        pub mod validation;
    }
    pub mod mainnet {
        pub mod buy_sell;
        pub mod setup;
    }
    pub mod flashbots {
        pub mod flashbot_main;
        pub mod submit_tx;
    }
    pub mod prepare_tx;
    pub mod token_price;
    pub mod tx_trait;
}
