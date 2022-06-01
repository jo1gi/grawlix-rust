mod options;
mod logging;

use std::{io::Write, process::exit};

use log::{info, error};
use logging::setup_logger;
use options::Config;
use structopt::StructOpt;
use thiserror::Error;
use grawlix::{
    error::GrawlixIOError,
    comic::Comic,
    metadata::Metadata,
    source::{Source, download_comics, download_comics_metadata}
};

#[derive(Debug, Error)]
/// Errors for Grawlix cli
pub enum CliError {
    #[error("Invalid input: {0}. Could not parse it as an url or a path")]
    Input(String),
    #[error(transparent)]
    Write(#[from] grawlix::error::GrawlixIOError),
    #[error(transparent)]
    Download(#[from] grawlix::error::GrawlixDownloadError),
    #[error("Could not create credentials from input")]
    InvalidCredentials,
    #[error("No Credentials found for source {0}")]
    MissingCredentials(String),
    #[error(transparent)]
    LogError(#[from] fern::InitError),
    #[error("Unknown error occurred")]
    Unknown,
}

type Result<T> = std::result::Result<T, CliError>;

#[tokio::main]
async fn main() {
    match do_stuff().await {
        Ok(_) => (),
        Err(e) => error!("{}", e)
    }
}

/// Create vector of comics from list of inputs
async fn load_inputs(inputs: &[String]) -> Result<Vec<Comic>> {
    let mut comics: Vec<Comic> = Vec::new();
    let re = regex::Regex::new(r"https?://.+\.[a-zA-Z0-9]+").unwrap();
    for i in inputs {
        let mut comic = if re.is_match(&i) {
            download_comics(&i).await?
        } else if std::path::Path::new(&i).exists() {
            vec![Comic::from_file(&i)?]
        } else {
            return Err(CliError::Input(i.to_string()))
        };
        comics.append(&mut comic);
    }
    return Ok(comics);
}

async fn create_authenticated_source(url: &str, _config: &Config) -> Result<Box<dyn Source>> {
    let source = grawlix::source::source_from_url(url)
        .or(Err(CliError::Input(url.to_string())))?;
    match source.name() {
        _ => Err(CliError::MissingCredentials(source.name()))
    }
}

async fn load_metadata(inputs: &[String], config: &Config) -> Result<Vec<Metadata>> {
    let mut all_metadata = Vec::new();
    for i in inputs {
        let mut source = create_authenticated_source(i, config).await?;
        all_metadata.append(&mut download_comics_metadata(&mut source, i).await.unwrap());
    }
    // TODO sort metadata
    Ok(all_metadata)
}

const PROGRESS_FILE: &str = ".grawlix-progress";

async fn do_stuff() -> Result<()> {
    // Loading options
    let args = options::Arguments::from_args();
    let config = options::load_options(&args).unwrap();
    setup_logger(args.log_level)?;
    // Loading comics
    let progress_file =  std::path::Path::new(PROGRESS_FILE);
    let comics = if progress_file.exists() {
        info!("Loading progress file");
        // Loading unfinished progress from last run of program
        let comics = serde_json::from_str(
            &std::fs::read_to_string(PROGRESS_FILE).map_err(|x| GrawlixIOError::from(x))?
        ).unwrap();
        // Removing temporary file
        match std::fs::remove_file(PROGRESS_FILE) {
            Ok(_) => (),
            Err(_) => error!("Could not remove progress file ({})", PROGRESS_FILE)
        }
        comics
    } else {
        info!("Searching for comics");
        let inputs = load_inputs(&args.inputs).await?;
        inputs
    };
    write_comics(comics, &config).await
}


/// Download comics and write them to disk
/// Will create a file with unfinished progress if a ctrl-c signal is recieved while running
async fn write_comics(comics: Vec<Comic>, config: &Config) -> Result<()> {
    info!("Found {} comics", comics.len());
    // Save progress on ctrl-c
    let comics = std::sync::Arc::new(comics);
    let comics_c = comics.clone();
    let progress = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let progress_c = progress.clone();
    tokio::spawn(async move {
        // Waiting for ctrl-c
        tokio::signal::ctrl_c().await.expect("failed to listen for event");
        // Creating file that describes the remainding progress
        let mut file = std::fs::File::create(PROGRESS_FILE).unwrap();
        let rest = &comics_c[progress_c.load(std::sync::atomic::Ordering::Relaxed)..];
        match file.write_all(serde_json::to_string(rest).unwrap().as_bytes()) {
            Ok(_) => info!("Saved progress to .grawlix-progress"),
            Err(_) => error!("Could not save progress file ({})", PROGRESS_FILE)
        };
        exit(0);
    });
    // Download comics
    for comic in comics.iter() {
        info!("Downloading {}", comic.title());
        comic.write(&config.output_template, &config.output_format).await?;
        progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}
