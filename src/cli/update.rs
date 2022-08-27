use crate::{
    CliError,
    utils::{self, get_all_links, write_comics},
    options::{Arguments, Config}
};
use grawlix::source::{ComicId, source_from_name, download_comics, get_all_ids, download_series_metadata};
use thiserror::Error;
use displaydoc::Display;
use log::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use std::io::Write;

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
    downloaded_issues: Vec<String>
}

type Result<T> = std::result::Result<T, UpdateError>;

/// Load updatefile from disk if it exists
fn load_updatefile(path: &str) -> Result<Vec<UpdateSeries>> {
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

fn write_updatefile(update_data: Vec<UpdateSeries>, path: &str) {
    let mut file = std::fs::File::create(path).unwrap();
    match file.write_all(serde_json::to_string(&update_data).unwrap().as_bytes()) {
        Ok(_) => (),
        Err(_) => error!("Could not save update file to {}", path)
    }
}

/// Add series to update file
pub async fn add(args: &Arguments, config: &Config, inputs: &Vec<String>) -> std::result::Result<(), CliError> {
    let links = get_all_links(args, inputs)?;
    let mut update_data = load_updatefile(&config.update_location)?;
    for link in links {
        let source = utils::get_source(&link, config).await?;
        let id = source.id_from_url(&link)?;
        debug!("Found id: {:?}", id);
        if let ComicId::Series(series_id) = &id {
            let client = source.create_client();
            let series_info = download_series_metadata(&client, &source, &id).await?;
            if !update_data.iter().any(|x| x.source == source.name() && &x.id == series_id) {
                update_data.push(UpdateSeries {
                    source: source.name(),
                    name: series_info.name.clone(),
                    id: series_id.to_string(),
                    downloaded_issues: Vec::new(),
                });
                info!("Added {}", &series_info.name);
            }
        } else {
            warn!("Can't add {} to update file since it is not a series", link);
        }
    }
    update_data.sort_by(|x, y| x.name.cmp(&y.name));
    write_updatefile(update_data, &config.update_location);
    Ok(())
}

pub fn list(config: &Config) -> std::result::Result<(), CliError> {
    let update_data = load_updatefile(&config.update_location)?;
    for series in update_data {
        println!("{}", series.name);
    }
    Ok(())
}


/// Update all files stored in updatefile
pub async fn update(config: &Config) -> std::result::Result<(), CliError> {
    let mut update_data = load_updatefile(&config.update_location)?;
    info!("Searching for updates");
    for series in &mut update_data {
        let source = source_from_name(&series.source)?;
        let client = source.create_client();
        let ids: Vec<ComicId> = get_all_ids(&client, ComicId::Series(series.id.clone()), &source).await?
            .into_iter()
            .filter(|x| !series.downloaded_issues.contains(x.inner()))
            .collect();
        if ids.len() == 0 {
            continue
        }
        info!("Retrieving data for {} comics from {}", ids.len(), series.name);
        let comics = download_comics(ids.clone(), &client, &source).await?;
        write_comics(comics, config).await?;
        let mut string_ids = ids.iter()
            .map(|x| x.inner().clone())
            .collect();
        series.downloaded_issues.append(&mut string_ids);
    }
    write_updatefile(update_data, &config.update_location);
    info!("Completed update");
    Ok(())
}
