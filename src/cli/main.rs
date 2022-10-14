mod options;
mod logging;
mod update;
mod utils;


use log::{info, error};
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
    /// Failed to read config file: {0}
    InvalidConfigFile(#[from] toml::de::Error),
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
    logging::setup_logger(args.log_level)?;
    let config: Config = options::load_options(&args)?;
    match &args.cmd {
        Command::Add { inputs } => update::add(&args, &config, inputs).await,
        Command::Download{ inputs } => download(inputs, &args, &config).await,
        Command::Info { inputs } => info(&args, &config, inputs).await,
        Command::List => update::list(&config),
        Command::Update => update::update(&config).await
    }
}


/// Download comics
async fn download(inputs: &Vec<String>, args: &Arguments, config: &Config) -> Result<()> {
    info!("Searching for comics");
    let links = utils::get_all_links(inputs, args)?;
    for link in links {
        let (source, client) = utils::get_source_from_url(&link, config).await?;
        let link_id = source.id_from_url(&link)?;
        let comicids = grawlix::source::get_all_ids(&source, &client, link_id).await?;
        utils::download_and_write_comics(&source, &client, &comicids, config).await;
    }
    Ok(())
}

/// Print comics to stdout
async fn info(args: &Arguments, config: &Config, inputs: &Vec<String>) -> Result<()> {
    let comics = utils::get_comics(args, config, inputs).await?;
    log::debug!("Found {} comics", comics.len());
    if config.json {
        println!("{}", serde_json::to_string_pretty(&comics).unwrap());
    } else {
        for comic in comics {
            logging::print_comic(&comic, config.json);
        }
    }
    Ok(())
}

