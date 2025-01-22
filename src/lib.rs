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

pub mod verify {
    pub mod openai {
        pub mod ai_submission;
        pub mod structs;
    }
    pub mod check_token_lock;
    pub mod detect_honeypot;
    pub mod etherscan_api;
    pub mod thegraph_api;
}

pub mod utils {
    pub mod logging;
    pub mod tx;
    pub mod type_conversion;
    pub mod web_scrapper;
}

pub mod data {
    pub mod api_counts;
    pub mod contracts;
    pub mod gas;
    pub mod nonce;
    pub mod portfolio;
    pub mod token_data;
    pub mod token_state_update;
    pub mod tokens;
}

pub mod token_tx {
    pub mod anvil;
    pub mod time_intervals;
    pub mod tx;
    pub mod validate;
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
        pub mod live_validation;
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
