use dotenv::dotenv;
use snipper::verify::{etherscan_api::get_source_code, openai_api::audit_token_contract};

// TEST ON BASE
#[tokio::test]
#[ignore]
async fn test_audit_token_contract() -> anyhow::Result<()> {
    dotenv().ok();
    const VIRTUALS: &str = "0x0b3e328455c4059EEb9e3f84b5543F74E24e7E1b";
    let source_code = get_source_code(VIRTUALS).await?;

    let audit = audit_token_contract(source_code).await?.unwrap();
    println!("{:#?}", audit);

    // assert!(!source_code.is_empty());

    Ok(())
}

// TEST ON BASE
#[tokio::test]
async fn test_audit_token_contract_2() -> anyhow::Result<()> {
    dotenv().ok();
    const SCAM_TOKEN: &str = "0x1f035d740FD128E3818a08D613bC4C2D8f8Fccee";
    let source_code = get_source_code(SCAM_TOKEN).await?;

    let audit = audit_token_contract(source_code).await?.unwrap();

    Ok(())
}
