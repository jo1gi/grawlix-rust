mod format;
pub mod read;
mod write;
use std::{collections::HashMap, str::FromStr};

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


#[derive(Debug, Deserialize, Serialize)]
pub struct Page {
    file_format: String,
    page_type: PageType,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PageType {
    /// Page on website
    Url(OnlinePage),
    /// Page in container
    Container(String),
}

/// Instructions on how to download a page
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct OnlinePage {
    /// Url of page
    url: String,
    /// Required headers for request
    headers: Option<HashMap<String, String>>,
    /// Encryption scheme of page
    encryption: Option<PageEncryptionScheme>
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PageEncryptionScheme {
    XOR(Vec<u8>)
}

impl Page {
    pub fn from_url(url: &str, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Url(OnlinePage {
                url: url.to_string(),
                ..Default::default()
            })
        }
    }

    pub fn from_url_with_headers(url: &str, headers: HashMap<String, String>, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Url(OnlinePage {
                url: url.to_string(),
                headers: Some(headers),
                encryption: None,
            })
        }
    }

    pub fn from_url_xor(url: &str, key: Vec<u8>, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Url(OnlinePage {
                url: url.to_string(),
                headers: None,
                encryption: Some(PageEncryptionScheme::XOR(key))
            })
        }
    }

    pub fn from_filename(filename: &str, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Container(filename.to_string())
        }
    }
}

impl OnlinePage {
    pub async fn download_page(&self, client: &reqwest::Client) -> Vec<u8> {
        let mut req = client.get(&self.url);
        if let Some(headers) = &self.headers {
            req = req.headers(headers.try_into().unwrap());
        }
        let resp = req.send().await.unwrap();
        let bytes = resp.bytes().await.unwrap().as_ref().into();
        match &self.encryption {
            Some(enc) => decrypt_page(bytes, enc),
            None => bytes
        }
    }
}

fn decrypt_page(bytes: Vec<u8>, enc: &PageEncryptionScheme) -> Vec<u8> {
    match enc {
        PageEncryptionScheme::XOR(key) => {
            bytes.iter()
                .zip(key.iter().cycle())
                .map(|(v, k)| v ^ k)
                .collect()
        }
    }
}
