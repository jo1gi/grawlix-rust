/// Functions for downloading comics
mod download;
/// Utility functions and macros for implementing `Source`
mod utils;
/// Implementations of `Source` for different sites
mod sites;

pub use download::*;

pub use sites::{source_from_name, source_from_url};

use crate::{
    error::GrawlixDownloadError as Error,
    metadata::Metadata,
    comic::Page
};
use reqwest::Client;
use serde::{Deserialize, Serialize};


/// Result type with `GrawlixDownloadError`
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
/// Id of comic or series on source
pub enum ComicId {
    Issue(String),
    IssueWithMetadata(String, Metadata),
    Other(String),
    OtherWithMetadata(String, Metadata),
    Series(String),
}

impl ComicId {
    pub fn inner(&self) -> &String {
        match self {
            ComicId::Issue(x)
            | ComicId::IssueWithMetadata(x, _)
            | ComicId::Other(x)
            | ComicId::OtherWithMetadata(x, _)
            | ComicId::Series(x) => x
        }
    }
}

/// Info about comic series
#[derive(Default)]
pub struct SeriesInfo {
    /// Name of series
    pub name: String,
    /// true if the series is ended false if not or unknown
    pub ended: bool,
}

/// Response from source.
pub enum SourceResponse<T> {
    /// New http request
    Request(Request<SourceResponse<T>>),
    /// Return value
    Value(T)
}

/// Http request(s) with a function to transform the data
pub struct Request<T> {
    /// Reqwest request
    requests: Vec<reqwest::RequestBuilder>,
    /// Function to parse response
    transform: Box<dyn Fn(&[bytes::Bytes]) -> Option<T>>,
}

/// Login credentials for source
pub enum Credentials {
    UsernamePassword(String, String),
    ApiKey(String),
}



/// Trait for interacting with comic book source
/// Trait object can be created with `source_from_url` function
#[async_trait::async_trait]
pub trait Source: Send {
    /// Name of source
    fn name(&self) -> String;

    /// Create `reqwest::Client` to use for all requests generated from source
    fn create_client(&self) -> reqwest::Client {
        download::create_default_client()
    }

    /// Converts an url to `ComicId`
    fn id_from_url(&self, url: &str) -> Result<ComicId>;

    /// Retrieves real id instead of `ComicId::Other`
    ///
    /// This is only meant to be called if the source returns the `ComicId::Other` type in
    /// `id_from_url` or `get_series_ids`.
    #[allow(unused_variables)]
    fn get_correct_id(&self, client: &Client, otherid: &ComicId) -> Result<SourceResponse<ComicId>> {
        unreachable!()
    }

    /// Retrieves `ComicId` for all comics in series
    /// `seriesid` has to be a `ComicId::Series`
    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<SourceResponse<Vec<ComicId>>>;

    /// Creates `SourceREsponse` to download comic metadata
    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>>;

    /// Creates `SourceResponse` to get metadata about series
    #[allow(unused_variables)]
    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>>;

    /// Downloads pages
    #[allow(unused_variables)]
    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Vec<Page>>> {
        Err(Error::PagesNotSupported(self.name()))
    }

    /// Returns `true` if authentication is needed to download metadata
    fn metadata_require_authentication(&self) -> bool {
        true
    }

    /// Returns `true` if authentication is needed to download pages
    fn pages_require_authentication(&self) -> bool {
        true
    }

    /// Authenticate with source using `creds`
    #[allow(unused_variables)]
    async fn authenticate(&mut self, client: &mut Client, creds: &Credentials) -> Result<()> {
        Ok(())
    }

}
