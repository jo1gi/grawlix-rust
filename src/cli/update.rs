use crate::{
    CliError,
    utils::{self, get_all_links, write_comics, get_source_from_name},
    options::{Arguments, Config}
};
use grawlix::source::{
    Source, ComicId, download_comics, get_all_ids, download_series_metadata
};
use thiserror::Error;
use displaydoc::Display;
use log::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use std::io::Write;
use reqwest::Client;

/// Errors for automatic updates
#[derive(Debug, Error, Display)]
pub enum UpdateError {
    /// {0} is not a series
    NotASeries(String),
    /// Could not load update file from {0}
    LoadUpdateFile(String),
}

/// Stores necassary information to update a series
#[derive(Deserialize, Serialize)]
struct UpdateSeries {
    source: String,
    name: String,
    id: String,
    #[serde(default = "Default::default")]
    ended: bool,
    downloaded_issues: Vec<String>
}

/// Load updatefile from disk if it exists
fn load_updatefile(path: &str) -> Result<Vec<UpdateSeries>, UpdateError> {
    if std::path::Path::new(&path).exists() {
        std::fs::read_to_string(&path)
            .ok()
            .map(|x| serde_json::from_str(&x).ok())
            .flatten()
            .ok_or(UpdateError::LoadUpdateFile(path.to_string()))
    } else {
        return Ok(Vec::new());
    }
}

fn write_updatefile(update_data: &Vec<UpdateSeries>, path: &str) {
    let mut file = std::fs::File::create(path).unwrap();
    match file.write_all(serde_json::to_string(&update_data).unwrap().as_bytes()) {
        Ok(_) => (),
        Err(_) => error!("Could not save update file to {}", path)
    }
}

/// Download `crate::source::SeriesInfo` for given series
async fn create_new_updateseries(source: &Box<dyn Source>, client: &Client, id: &ComicId) -> Result<UpdateSeries, CliError> {
    let series_info = download_series_metadata(client, source, id).await?;
    Ok(UpdateSeries {
        source: source.name(),
        name: series_info.name.clone(),
        ended: series_info.ended,
        id: id.inner().to_string(),
        downloaded_issues: Vec::new()
    })
}

/// Add series to update file
pub async fn add(args: &Arguments, config: &Config, inputs: &Vec<String>) -> std::result::Result<(), CliError> {
    let links = get_all_links(args, inputs)?;
    let mut update_data = load_updatefile(&config.update_location)?;
    for link in links {
        let (source, client) = utils::get_source_from_url(&link, config).await?;
        let id = source.id_from_url(&link)?;
        debug!("Found id: {:?}", id);
        if let ComicId::Series(_) = &id {
            let update_series = create_new_updateseries(&source, &client, &id).await?;
            if !update_data.iter().any(|x| x.source == update_series.source && x.id == update_series.id) {
                info!("Added {}", &update_series.name);
                update_data.push(update_series);
            }
        } else {
            warn!("Can't add {} to update file since it is not a series", link);
        }
    }
    update_data.sort_by(|x, y| x.name.cmp(&y.name));
    write_updatefile(&update_data, &config.update_location);
    Ok(())
}

/// Print all series in updatefile
pub fn list(config: &Config) -> Result<(), CliError> {
    let update_data = load_updatefile(&config.update_location)?;
    for series in update_data {
        println!("{}", series.name);
    }
    Ok(())
}

/// Update info about series for all series in update_data
async fn update_series_info(mut update_data: Vec<UpdateSeries>, config: &Config) -> Result<Vec<UpdateSeries>, CliError> {
    for series in &mut update_data {
        debug!("Updating info for {} ({})", series.name, series.id);
        let (source, client) = get_source_from_name(&series.source, config).await?;
        let new_data = create_new_updateseries(&source, &client, &ComicId::Series(series.id.clone())).await?;
        series.name = new_data.name;
        series.ended = new_data.ended;
    }
    Ok(update_data)
}

/// Downloads new comics for all series in `update_data`
async fn download_new_comics(update_data: &mut Vec<UpdateSeries>, config: &Config) -> Result<(), CliError> {
    for series in update_data {
        info!("Searching for updates in {}", series.name);
        let (source, client) = get_source_from_name(&series.source, config).await?;
        // Finding new ids
        let ids: Vec<ComicId> = get_all_ids(&client, ComicId::Series(series.id.clone()), &source).await?
            .into_iter()
            .filter(|x| !series.downloaded_issues.contains(x.inner()))
            .collect();
        // Downloading new comics
        if ids.len() == 0 {
            continue
        }
        info!("Retrieving data for {} comics from {}", ids.len(), series.name);
        let comics = download_comics(ids.clone(), &client, &source).await?;
        write_comics(comics, config).await?;
        // Adding new ids to update file
        let mut string_ids = ids.iter()
            .map(|x| x.inner().clone())
            .collect();
        series.downloaded_issues.append(&mut string_ids);
    }
    Ok(())
}

/// Remove all series that have ended
fn remove_ended_series(update_data: Vec<UpdateSeries>) -> Vec<UpdateSeries> {
    update_data.into_iter()
        .filter(|series| !series.ended)
        .collect()
}

/// Update all files stored in updatefile
pub async fn update(config: &Config) -> Result<(), CliError> {
    let mut update_data = load_updatefile(&config.update_location)?;
    if config.update_series_info {
        info!("Updating series info");
        update_data = update_series_info(update_data, config).await?;
    }
    download_new_comics(&mut update_data, config).await?;
    let update_data = remove_ended_series(update_data);
    write_updatefile(&update_data, &config.update_location);
    info!("Completed update");
    Ok(())
}
