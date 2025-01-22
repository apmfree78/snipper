use dotenv::dotenv;
use snipper::verify::etherscan_api::{get_source_code, EtherscanResponse, TokenInfo};

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_etherscan_api() -> anyhow::Result<()> {
    dotenv().ok();
    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";

    let source_code = get_source_code(VIRTUALS).await?;
    println!("source code => {}", source_code);

    assert!(!source_code.is_empty());

    Ok(())
}

// TEST ON MAINNET
#[tokio::test]
#[ignore]
async fn test_etherscan_api_2() -> anyhow::Result<()> {
    dotenv().ok();
    const QUALIFY_USER: &str = "0x5F0604C368B433e829905dFcB14f23B6f077e885";

    let source_code = get_source_code(QUALIFY_USER).await?;

    println!("source code => {}", source_code);

    assert!(source_code.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_parse() -> anyhow::Result<()> {
    let json_data = r#"
    {
       "status":"1",
       "message":"OK",
       "result":[
          {
             "contractAddress":"0xc46fff769c2d1766db71606c8549de4db4c65bb2",
             "tokenName":"🧩PUZZLE",
             "symbol":"🧩PUZZLE",
             "divisor":"18",
             "tokenType":"",
             "totalSupply":"1000000000000000000000000000",
             "blueCheckmark":"false",
             "description":"",
             "website":"",
             "email":"",
             "blog":"",
             "reddit":"",
             "slack":"",
             "facebook":"",
             "twitter":"",
             "bitcointalk":"",
             "github":"",
             "telegram":"",
             "wechat":"",
             "linkedin":"",
             "discord":"",
             "whitepaper":"",
             "tokenPriceUSD":"0.0000000000",
             "image":""
          }
       ]
    }
    "#;

    let parsed: EtherscanResponse<TokenInfo> = serde_json::from_str(json_data).unwrap();
    println!("Parsed: {:#?}", parsed);

    Ok(())
}
