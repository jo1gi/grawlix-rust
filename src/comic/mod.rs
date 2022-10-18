mod format;
mod page;
pub mod read;
mod write;

pub use page::*;

use crate::metadata::Metadata;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Comic {
    pub metadata: Metadata,
    pub pages: Vec<Page>,
}

impl Comic {
    /// Create new default `Comic`
    pub fn new() -> Self {
        Default::default()
    }

    /// Return title of comic or "UNKNOWN" if title is None
    pub fn title<'a>(&'a self) -> &'a str {
        match &self.metadata.title {
            Some(title) => title,
            None => "UNKNOWN"
        }
    }

}

#[derive(Deserialize, Debug, Clone)]
/// Indicator for output format
pub enum ComicFormat {
    CBZ,
    Dir,
}

impl Default for ComicFormat {
    fn default() -> Self {
        Self::CBZ
    }
}

impl FromStr for ComicFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cbz" | "zip" => Ok(Self::CBZ),
            "dir" | "folder" => Ok(Self::Dir),
            _ => Err("Could not parse comic format type")
        }
    }
}
