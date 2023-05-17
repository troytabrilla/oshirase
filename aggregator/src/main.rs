use aggregator::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Aggregate anime and manga data from different sources."
)]
struct Cli {
    #[arg(short, long, help = "Path to config.toml")]
    config: Option<String>,

    #[arg(short, long, help = "Skip cache check")]
    skip_cache: bool,

    #[arg(short, long, help = "Print results")]
    print: bool,

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

    // @todo Set up a separate chron job (in docker?) to send an aggregator job to the queue every x minutes
    let options = RunOptions {
        skip_cache: Some(cli.skip_cache),
        extract_options: Some(ExtractOptions {
            skip_cache: Some(cli.skip_cache),
        }),
    };

    let mut aggregator = Aggregator::new(&config).await;

    if cli.worker_mode {
        let mut worker = Worker::new(&mut aggregator);
        worker.run(Some(&options)).await;
    } else {
        let data = aggregator.run(Some(&options)).await?;

        if cli.print {
            println!("{:?}", data);
        }
    }

    Ok(())
}
