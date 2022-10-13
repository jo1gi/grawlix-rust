mod comicrack;
#[cfg(test)]
mod tests;

use crate::error::GrawlixIOError as Error;
use std::{fmt, io::Read, str::FromStr};
use serde::{Deserialize, Serialize};

/// Stores metadata about a comic book
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
pub struct Metadata {
    /// Title of comic
    pub title: Option<String>,
    /// List of authors and artists
    pub authors: Vec<Author>,
    /// Name of publisher
    pub publisher: Option<String>,
    /// Series name
    pub series: Option<String>,
    /// Issue number
    pub issue_number: Option<u32>,
    /// Relase year
    pub year: Option<u32>,
    /// Relase month (1 indexed)
    pub month: Option<u32>,
    /// Relase day
    pub day: Option<u32>,
    /// Reading Direction
    pub reading_direction: ReadingDirection,
    /// Database identifiers
    pub identifiers: Vec<Identifier>,
    /// Description
    pub description: Option<String>,
    /// The source the comic has been downloaded from
    pub source: Option<String>,
}

impl Metadata {

    /// Date as string
    pub fn date(&self) -> Option<String> {
        if let (Some(year), Some(month), Some(day)) = (self.year, self.month, self.day) {
            Some(format!("{}-{}-{}", year, month, day))
        } else {
            None
        }
    }

    /// Export metadata in all available formats
    pub fn export_all(&self) -> Result<Vec<(&str, String)>, Error> {
        Ok(vec![
            ("comicinfo.xml", self.comicrack()
                .or(Err(Error::MetadataExport("Comicrack".to_string())))?),
            ("grawlix.json", serde_json::to_string(&self)
                .or(Err(Error::MetadataExport("Grawlix".to_string())))?)
        ])
    }

    pub fn from_metadata_file<R: Read>(name: &str, mut r: R) -> Option<Self> {
        match name {
            "grawlix.json" => {
                let mut buffer = String::new();
                r.read_to_string(&mut buffer).ok()?;
                serde_json::from_str(&buffer).ok()
            },
            "comicinfo.xml" => Some(Self::from_comicrack(r)),
            _ => None,
        }
    }
}

/// Author of comic book
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Author {
    /// Name of author
    pub name: String,
    /// Type of author or artist
    pub author_type: AuthorType,
}

/// Comic book author type
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AuthorType {
    Writer,
    Penciller,
    Inker,
    Colorist,
    Letterer,
    CoverArtist,
    Editor,
    Other
}

impl fmt::Display for AuthorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AuthorType::Writer => "Writer",
            AuthorType::Penciller => "Penciller",
            AuthorType::Inker => "Inker",
            AuthorType::Colorist => "Colorist",
            AuthorType::Letterer => "Letterer",
            AuthorType::CoverArtist => "CoverArtist",
            AuthorType::Editor => "Editor",
            AuthorType::Other => "Other",
        };
        f.write_str(s)
    }
}

impl From<String> for AuthorType {
    fn from(s: String) -> Self {
        let lower = s.to_ascii_lowercase();
        if lower.contains("cover") {
            return AuthorType::CoverArtist
        }
        match lower.as_str() {
            "writer" => AuthorType::Writer,
            "penciller" => AuthorType::Penciller,
            "inks" | "inker" => AuthorType::Inker,
            "colors" | "colorist" => AuthorType::Colorist,
            "letterer" => AuthorType::Letterer,
            "coverartist" => AuthorType::CoverArtist,
            "editor" => AuthorType::Editor,
            _ => AuthorType::Other,
        }
    }
}

impl From<&str> for AuthorType {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

/// Reading direction of book
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ReadingDirection {
    LeftToRight,
    RightToLeft,
}

impl Default for ReadingDirection {
    fn default() -> Self {
        ReadingDirection::LeftToRight
    }
}

impl FromStr for ReadingDirection {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();
        let direction = match lower.as_str() {
            "ltr" => Self::LeftToRight,
            "rtl" => Self::RightToLeft,
            _ => return Err(()),
        };
        Ok(direction)
    }
}

impl TryFrom<&str> for ReadingDirection {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

/// Comic book identifier
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Identifier {
    pub source: String,
    pub id: String,
}

/// Convert a string in the form "year-month-day" to a tuple with those values
pub fn date_from_str(date: &str) -> Option<(u32, u32, u32)> {
    let tmp: Vec<u32> = date.split("-")
        .filter_map(|x| x.parse::<u32>().ok())
        .collect();
    Some((*tmp.get(0)?, *tmp.get(1)?, *tmp.get(2)?))
}
