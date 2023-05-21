use aggregator::Aggregator;
use aggregator::Config;
use aggregator::Result;
use aggregator::RunOptions;
use aggregator::Worker;

use clap::Parser;

#[derive(Parser)]
#[command(
    version,
    about = "Aggregate anime and manga data from different sources."
)]
struct Cli {
    #[arg(short, long, help = "Path to config.toml")]
    config: Option<String>,

    #[arg(short, long, help = "Print results")]
    print: bool,

    #[arg(short, long, help = "AniList user ID")]
    user_id: Option<u64>,

    #[arg(short, long, help = "Run in worker mode")]
    worker_mode: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = match cli.config {
        Some(config) => Config::from_file(&config),
        None => Config::default(),
    };

    let aggregator = Aggregator::new(&config);

    if cli.worker_mode {
        let worker = Worker::new(&aggregator);
        worker.run().await;
    } else {
        let options = RunOptions {
            user_id: cli.user_id,
        };
        let data = aggregator.run(Some(options)).await?;

        if cli.print {
            println!("{:?}", data);
        }
    }

    Ok(())
}
