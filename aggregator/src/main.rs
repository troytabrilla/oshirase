use aggregator::config::Config;
use aggregator::Aggregator;
use aggregator::Result;

use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let config = if args.len() < 2 {
        println!("Using default config file.");
        Config::default()
    } else {
        println!("Using config file {}.", args[1]);
        Config::from_file(&args[1])
    };

    Aggregator::new(&config).await.run(None).await?;

    Ok(())
}
