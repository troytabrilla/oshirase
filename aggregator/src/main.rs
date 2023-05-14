use aggregator::*;
use config::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Aggregate anime and manga data from different sources."
)]
struct Cli {
    #[arg(short, long, help = "Path to config.toml")]
    config: Option<String>,

    #[arg(short, long, help = "Disable caching")]
    dont_cache: bool,

    #[arg(short, long, help = "Print results")]
    print: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = match cli.config {
        Some(config) => Config::from_file(&config),
        None => Config::default(),
    };

    let options = RunOptions {
        dont_cache: Some(cli.dont_cache),
        extract_options: Some(ExtractOptions {
            dont_cache: Some(cli.dont_cache),
        }),
    };

    let data = Aggregator::new(config).await.run(Some(options)).await?;

    if cli.print {
        println!("{:?}", data);
    }

    Ok(())
}
