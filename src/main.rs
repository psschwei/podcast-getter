mod cli;
mod config;
mod download;
mod feed;
mod image;
mod state;
mod tagger;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "podcast-getter")]
#[command(about = "A CLI utility for downloading podcasts from RSS feeds", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable debug logging
    #[arg(global = true, short, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Download new episodes from all configured podcasts
    Download {
        /// Maximum number of episodes to download per podcast
        #[arg(short, long)]
        max_episodes: Option<usize>,
    },

    /// Add a new podcast feed
    #[command(about = "Add a new podcast to the config")]
    Add {
        /// URL of the RSS feed
        #[arg(value_name = "URL")]
        url: String,

        /// Name for the podcast (optional)
        #[arg(short, long)]
        name: Option<String>,

        /// Output directory for downloads (optional)
        #[arg(short, long)]
        output_dir: Option<PathBuf>,
    },

    /// List all configured podcasts
    List,

    /// Show last-check timestamps for all podcasts
    Status,

    /// Update a specific podcast feed
    #[command(about = "Check and download new episodes from a specific podcast")]
    UpdateFeed {
        /// Name of the podcast to update
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Create example config file
    #[command(about = "Generate an example config file")]
    InitConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter_level = if cli.debug {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter_level)),
        )
        .init();

    match cli.command {
        Commands::Download { max_episodes } => {
            cli::download_all_podcasts(max_episodes).await?;
        }
        Commands::Add { url, name, output_dir } => {
            cli::add_podcast(url, name, output_dir)?;
        }
        Commands::List => {
            cli::list_podcasts()?;
        }
        Commands::Status => {
            cli::show_status()?;
        }
        Commands::UpdateFeed { name } => {
            cli::update_feed(name).await?;
        }
        Commands::InitConfig => {
            config::Config::create_example()?;
        }
    }

    Ok(())
}
