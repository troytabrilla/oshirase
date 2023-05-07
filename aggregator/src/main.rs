use aggregator::Aggregator;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Aggregator::default().aggregate().await?;

    Ok(())
}
