use ethers::types::{Log, H256, U256};
use hex_literal::hex;
use snipper::events;

#[test]
#[ignore]
fn decode_pair_event() -> anyhow::Result<()> {
    let topic_1: H256 =
        "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9".parse()?;
    let topic_2: H256 =
        "0x00000000000000000000000042a79d4ddde00955d15bb18b7983388c16012e61".parse()?;
    let topic_3: H256 =
        "0x000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".parse()?;

    let log = Log {
    address: "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f".parse()?, 
    topics:vec![ topic_1,topic_2, topic_3 ],
    data: ethers::types::Bytes::from(hex!(
    "000000000000000000000000d7f185d0bff9e41b3e454c7fda07b4a4fe1911c5000000000000000000000000000000000000000000000000000000000005fff9"
    )),
    block_hash: Some(
        "0xba7a9f4f4b0f13cc885cb10cc06d328ba84123aca7408ecb00fdbc6592798943".parse()?
    ),
    block_number: Some(
        ethers::types::U64::from(21396064)
    ),
    transaction_hash: Some(
        "0x271ad42cbb1ede13dc7062f6330137dff0f77637b5c04e452760bed9c2bc052e".parse()?
    ),
    transaction_index: Some(
        ethers::types::U64::from(1)
    ),
    log_index: Some(
        U256::from(3),
    ),
    transaction_log_index: None,
    log_type: None,
    removed: Some(
        false,
    ),
    };

    let pair_created_event = events::decode_pair_created_event(&log)?;

    println!("pair created event {:#?}", pair_created_event);

    Ok(())
}
