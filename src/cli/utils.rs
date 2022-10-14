use crate::{
    CliError, Result,
    logging,
    options::{Arguments, Config, SourceData}
};
use grawlix::{
    error::GrawlixIOError,
    comic::Comic,
    source::{
        self,
        Source, ComicId, source_from_url, get_all_ids, download_comics,
        source_from_name, comic_from_comicid
    }
};
use reqwest::Client;
use futures::{StreamExt, stream};

/// Get settings for source from config
fn get_source_settings(source: &Box<dyn Source>, config: &Config) -> Option<SourceData> {
    match source.name().as_str() {
        "DC Universe Infinite" => config.dcuniverseinfinite.clone(),
        "Izneo" => config.izneo.clone(),
        "Marvel" => config.marvel.clone(),
        _ => None
    }
}

fn load_cookies(source: &Box<dyn Source>, clientbuilder: &mut source::ClientBuilder, config: &Config) {
    log::debug!("Adding cookies to clientbuilder");
    if let Some(sourcedata) = get_source_settings(&source, config) {
        if let Some(cookies) = sourcedata.cookies {
            for (key, value) in cookies {
                clientbuilder.add_cookie(key, value);
            }
        }
    }
}

/// Authenticate `source` with credentials from `config`
pub async fn authenticate_source(source: &mut Box<dyn Source>, client: &mut Client, config: &Config) -> Result<()> {
    if let Some(sourcedata) = get_source_settings(&source, config) {
        if let Ok(credentials) = sourcedata.try_into() {
            log::debug!("Authenticating source");
            source.authenticate(client, &credentials).await?;
        }
    }
    Ok(())
}

/// Create source from dynamic method and authenticate it if credentials are available
async fn get_source<F>(method: &F, param: &str, config: &Config) -> Result<(Box<dyn Source>, Client)>
where
    F: Fn(&str) -> std::result::Result<Box<dyn Source>, grawlix::error::GrawlixDownloadError>,
{
    let mut source = method(param)?;
    let mut clientbuilder = source.client_builder();
    load_cookies(&source, &mut clientbuilder, config);
    let mut client = clientbuilder.to_reqwest_client();
    if source.requires_authentication() {
        authenticate_source(&mut source, &mut client, config).await?;
    }
    Ok((source, client))
}

/// Create source from url and authenticate if credentials are available
pub async fn get_source_from_url(url: &str, config: &Config) -> Result<(Box<dyn Source>, Client)> {
    get_source(&source_from_url, url, config).await
}

/// Create source from name of source and authenticate if credentials are available
pub async fn get_source_from_name(name: &str, config: &Config) -> Result<(Box<dyn Source>, Client)> {
    get_source(&source_from_name, name, config).await
}

async fn download_comics_from_url(url: &str, config: &Config) -> Result<Vec<Comic>> {
    let (source, client) = get_source_from_url(url, config).await?;
    let comicid = source.id_from_url(url)?;
    log::debug!("Got id from url: {:?}", comicid);
    let all_ids = get_all_ids(&source, &client, comicid).await?;
    let comics = download_comics(all_ids, &client, &source).await?;
    Ok(comics)
}

/// Create vector of comics from list of inputs
async fn load_inputs(inputs: &[String], config: &Config) -> Result<Vec<Comic>> {
    let mut comics: Vec<Comic> = Vec::new();
    let re = regex::Regex::new(r"https?://.+\.[a-zA-Z0-9]+").unwrap();
    for i in inputs {
        let mut comic = if re.is_match(&i) {
            download_comics_from_url(&i, config).await?
        } else if std::path::Path::new(&i).exists() {
            vec![Comic::from_file(&i)?]
        } else {
            return Err(CliError::Input(i.to_string()))
        };
        comics.append(&mut comic);
    }
    return Ok(comics);
}


/// Load all links from a file
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
pub fn get_all_links(inputs: &Vec<String>, args: &Arguments) -> Result<Vec<String>> {
    let mut x = inputs.clone();
    if let Some(link_file) = &args.file {
        x.append(&mut load_links_from_file(link_file)?);
    }
    return Ok(x);
}


/// Returns a list of comics based on arguments
pub async fn get_comics(args: &Arguments, config: &Config, inputs: &Vec<String>) -> Result<Vec<Comic>> {
    let links = get_all_links(inputs, args)?;
    if links.len() > 0 {
        log::info!("Searching for comics");
        Ok(load_inputs(&links, config).await?)
    } else {
        Ok(Vec::new())
    }
}

/// Download data about all comics and write them to disk
pub async fn download_and_write_comics(source: &Box<dyn Source>, client: &Client, comicids: &Vec<ComicId>, config: &Config) {
    stream::iter(comicids.clone())
        .map(|comicid| comic_from_comicid(&source, &client, comicid))
        .buffered(5)
        .for_each(|comic| async {
            match comic {
                Ok(x) => write_comic(&x, client, config).await.unwrap(),
                Err(e) => {
                    log::info!("Failed to download comic info: {}", e);
                },
            }
        })
        .await;
}

pub async fn write_comic(comic: &Comic, client: &Client, config: &Config) -> Result<()> {
    // Creating output path
    let path = comic.format(&config.output_template)?;
    // Checking if file already exists if overwrite is not enabled
    if !config.overwrite && std::path::Path::new(&path).exists() {
        log::info!("Skipping {} (File already exists)", comic.title());
        // Downloading comic
    } else {
        log::info!("Downloading {}", comic.title());
        if config.info {
            logging::print_comic(comic, config.json);
        }
        comic.write(&path, &config.output_format, client).await?;
    }
    Ok(())
}
