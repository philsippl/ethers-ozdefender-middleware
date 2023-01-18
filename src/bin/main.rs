use std::{env, str::FromStr};

use ethers::{
    abi::Address,
    prelude::MiddlewareBuilder,
    providers::{Http, Middleware, Provider},
    types::TransactionRequest,
};
use ethers_ozdefender_middleware::OzRelayerMiddleware;
use eyre::Result;
use tokio::time::Instant;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // API_KEY and API_SECRET are obtained from the Defender dashboard
    let api_key = env::var("API_KEY").expect("API_KEY is not set");
    let api_secret = env::var("API_SECRET").expect("API_SECRET is not set");

    // dummy tx
    let to = Address::from_str("0x00000000219ab540356cBB839Cbe05303d7705Fa")?;
    let tx = TransactionRequest::new().to(to).value(1).gas(100000);

    // init oz middleware
    let provider = Provider::<Http>::try_from("http://localhost")?
        .wrap_into(|s| OzRelayerMiddleware::new(s, api_key, api_secret).unwrap());

    for i in 0..5 {
        let start = Instant::now();
        let pending_tx = provider.send_transaction(tx.clone(), None).await?;
        let tx_id = Uuid::parse_str(&format!("{:x}", pending_tx.tx_hash())[32..])?;

        println!(
            "Sending transaction {} (id: {}) took {:?}",
            i,
            tx_id,
            start.elapsed()
        );
    }

    Ok(())
}
