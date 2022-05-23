mod format;
pub mod read;
mod write;
use serde::{Deserialize, Serialize};

use crate::metadata::Metadata;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Comic {
    pub metadata: Metadata,
    pub pages: Vec<Page>,
    #[serde(skip_serializing,skip_deserializing)]
    pub container: Option<Container>
}

#[derive(Debug)]
pub struct Container {
    path: String,
    format: ComicFormat,
}

impl Container {
    pub fn cbz(path: &str) -> Self {
        Self {
            path: path.to_string(),
            format: ComicFormat::CBZ
        }
    }
}

#[derive(Debug)]
pub enum ComicFormat {
    CBZ,
}

// async fn download_page(url: &str) -> bytes::Bytes {
//     // TODO Remove unwrap
//     client.get(url).send().await.unwrap().bytes().await.unwrap()
// }

impl Comic {
    /// Create new default `Comic`
    pub fn new() -> Self {
        Default::default()
    }

    /// Return title of comic or "UNKNOWN" if title is None
    pub fn title<'a>(&'a self) -> &'a str {
        match &self.metadata.title {
            Some(title) => title,
            None => "UNKOWN"
        }
    }

}


#[derive(Debug, Deserialize, Serialize)]
pub struct Page {
    file_format: String,
    page_type: PageType,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PageType {
    /// Page on website
    Url(String),
    /// Page in container
    Container(String),
}

impl Page {
    pub fn from_url(url: &str, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Url(url.to_string())
        }
    }

    pub fn from_filename(url: &str, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Container(url.to_string())
        }
    }
}
