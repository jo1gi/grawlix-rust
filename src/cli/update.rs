use crate::{
    CliError,
    utils::{self, get_all_links},
    options::{Arguments, Config}
};
use grawlix::source::ComicId;
use thiserror::Error;
use displaydoc::Display;
use log::{info, warn, error};
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
    series_id: String,
    downloaded_issues: Vec<ComicId>
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

/// Add series to update file
pub async fn add(args: &Arguments, config: &Config, inputs: &Vec<String>) -> std::result::Result<(), CliError> {
    let links = get_all_links(args, inputs)?;
    let mut update_data = load_updatefile(&config.update_location)?;
    for link in links {
        let source = utils::get_source(&link, config).await?;
        let id = source.id_from_url(&link)?;
        if let ComicId::Series(series_id) = id {
            if !update_data.iter().any(|x| x.source == source.name() && x.series_id == series_id) {
                update_data.push(UpdateSeries {
                    source: source.name(),
                    series_id,
                    downloaded_issues: Vec::new(),
                });
                info!("Added series from {}", link);
            }
        } else {
            warn!("Can't add {} to update file since it is not a series", link);
        }
    }
    let mut file = std::fs::File::create(&config.update_location).unwrap();
    match file.write_all(serde_json::to_string(&update_data).unwrap().as_bytes()) {
        Ok(_) => (),
        Err(_) => error!("Could not save update file to {}", config.update_location)
    }
    Ok(())
}
