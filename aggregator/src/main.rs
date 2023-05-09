use aggregator::Aggregator;
use aggregator::Result;

#[tokio::main]
async fn main() -> Result<()> {
    Aggregator::default().run().await?;

    Ok(())
}
