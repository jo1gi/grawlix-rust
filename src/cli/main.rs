mod options;
mod logging;
mod update;
mod utils;


use log::{info, error};
use logging::setup_logger;
use options::{Arguments, Command, Config};
use structopt::StructOpt;
use thiserror::Error;
use displaydoc::Display;

#[derive(Debug, Error, Display)]
/// Errors for Grawlix cli
pub enum CliError {
    /// Invalid input: {0}. Could not parse it as an url or a path
    Input(String),
    /// Could not find file: {0}
    FileNotFound(String),
    /// {0}
    Write(#[from] grawlix::error::GrawlixIOError),
    /// {0}
    Download(#[from] grawlix::error::GrawlixDownloadError),
    /// {0}
    Update(#[from] update::UpdateError),
    /// Could not create credentials from input
    InvalidCredentials,
    /// No Credentials found for source {0}
    MissingCredentials(String),
    /// {0}
    LogError(#[from] fern::InitError),
    /// Unknown error occurred
    Unknown,
}


type Result<T> = std::result::Result<T, CliError>;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => (),
        Err(e) => error!("{}", e)
    }
}

async fn run() -> Result<()> {
    // Loading options
    let args = Arguments::from_args();
    let config: Config = options::load_options(&args).unwrap();
    setup_logger(args.log_level)?;
    match &args.cmd {
        Command::Add { inputs } => update::add(&args, &config, inputs).await,
        Command::Download{ inputs } => download(&args, &config, inputs).await,
        Command::Info { inputs } => info(&args, &config, inputs).await,
        Command::List => update::list(&config),
        Command::Update => update::update(&config).await
    }
}


/// Download comics
async fn download(args: &Arguments, config: &Config, inputs: &Vec<String>) -> Result<()> {
    let comics = utils::get_comics(args, config, inputs).await?;
    info!("Found {} {}", comics.len(), if comics.len() > 1 { "comics" } else { "comic" });
    if comics.len() > 0 {
        utils::write_comics(comics, config).await?;
    }
    Ok(())
}

/// Print comics to stdout
async fn info(args: &Arguments, config: &Config, inputs: &Vec<String>) -> Result<()> {
    let comics = utils::get_comics(args, config, inputs).await?;
    if config.json {
        println!("{}", serde_json::to_string_pretty(&comics).unwrap());
    } else {
        for comic in comics {
            logging::print_comic(&comic, config.json);
        }
    }
    Ok(())
}

