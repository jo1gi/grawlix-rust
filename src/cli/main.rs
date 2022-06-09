mod options;
mod logging;

use std::{io::Write, process::exit, sync::{Arc, atomic::AtomicUsize}};

use log::{info, error};
use logging::setup_logger;
use options::Config;
use structopt::StructOpt;
use thiserror::Error;
use displaydoc::Display;
use grawlix::{
    error::GrawlixIOError,
    comic::Comic,
    source::download_comics
};

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

const PROGRESS_FILE: &str = ".grawlix-progress";

async fn do_stuff() -> Result<()> {
    // Loading options
    let args = options::Arguments::from_args();
    let config: Config = options::load_options(&args).unwrap();
    setup_logger(args.log_level)?;
    // Downloading comics
    let comics = get_comics(&args).await?;
    write_comics(comics, &config).await
}

fn load_links_from_file(link_file: &std::path::PathBuf) -> Result<Vec<String>> {
    if link_file.exists() {
        let links = std::fs::read_to_string(link_file)
            .map_err(|x| GrawlixIOError::from(x))?
            .lines()
            .map(String::from)
            .collect();
        Ok(links)
    } else {
        Err(CliError::FileNotFound(link_file.to_str().ok_or(CliError::Unknown)?.to_string()))
    }
}

/// Return all links from arguments, files, and pipe
fn get_all_links(args: &options::Arguments) -> Result<Vec<String>> {
    let mut x = args.inputs.clone();
    if let Some(link_file) = &args.file {
        x.append(&mut load_links_from_file(link_file)?);
    }
    return Ok(x);
}


/// Returns a list of comics based on arguments
async fn get_comics(args: &options::Arguments) -> Result<Vec<Comic>> {
    let progress_file =  std::path::Path::new(PROGRESS_FILE);
    if progress_file.exists() {
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
        Ok(comics)
    } else {
        info!("Searching for comics");
        Ok(load_inputs(&get_all_links(args)?).await?)
    }
}

/// Setup thread that listens for and handles ctrl-c signal
fn setup_ctrlc(comics: Arc<Vec<Comic>>, progress: Arc<AtomicUsize>, config: &Config) {
    let output_template = config.output_template.clone();
    tokio::spawn(async move {
        // Waiting for ctrl-c
        tokio::signal::ctrl_c().await.expect("failed to listen for event");
        // Creating file that describes the remainding progress
        let mut file = std::fs::File::create(PROGRESS_FILE).unwrap();
        let rest = &comics[progress.load(std::sync::atomic::Ordering::Relaxed)..];
        match file.write_all(serde_json::to_string(rest).unwrap().as_bytes()) {
            Ok(_) => info!("Saved progress to .grawlix-progress"),
            Err(_) => error!("Could not save progress file ({})", PROGRESS_FILE)
        };
        // Removing up unfinished file
        let unfinished_path = rest.get(0)
            .map(|x| x.format(&output_template).ok())
            .flatten();
        if let Some(x) = unfinished_path {
            match std::fs::remove_file(&x) {
                Ok(_) => (),
                Err(_) => error!(
                    "Could not remove unfinished file from downloading: {}",
                    &x
                )
            }
        }
        exit(0);
    });
}

/// Download comics and write them to disk
/// Will create a file with unfinished progress if a ctrl-c signal is recieved while running
async fn write_comics(comics: Vec<Comic>, config: &Config) -> Result<()> {
    info!("Found {} comics", comics.len());
    // Save progress on ctrl-c
    let comics = std::sync::Arc::new(comics);
    let progress = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    setup_ctrlc(comics.clone(), progress.clone(), config);
    // Download each comic
    for comic in comics.iter() {
        // Creating output path
        let path = comic.format(&config.output_template)?;
        // Checking if file already exists if overwrite is not enabled
        if !config.overwrite && std::path::Path::new(&path).exists() {
            info!("Skipping {} (File already exists)", comic.title());
        // Downloading comic
        } else {
            info!("Downloading {}", comic.title());
            comic.write(&path, &config.output_format).await?;
        }
        progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}
