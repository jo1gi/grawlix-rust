mod download;
/// Utility functions and macros for implementing `Source`
mod utils;

mod flipp;
mod leagueoflegends;
mod mangaplus;
mod marvel;
mod webtoon;

use crate::{
    error::GrawlixDownloadError as Error,
    metadata::Metadata,
    comic::Page
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
pub use download::{
    download_comics_from_url, download_comics, download_comics_metadata, create_default_client, get_all_ids, download_series_metadata
};

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
pub struct SeriesInfo {
    /// Name of series
    pub name: String,
}

/// Response from source.
pub enum SourceResponse<T> {
    /// New http request
    Request(Request<T>),
    /// Return value
    Value(T)
}

/// Http request(s) with a function to transform the data
pub struct Request<T> {
    /// Reqwest request
    requests: Vec<reqwest::Request>,
    /// Function to parse response
    transform: Box<dyn Fn(&[bytes::Bytes]) -> Option<T>>,
}

/// Login credentials for source
pub enum Credentials {
    UsernamePassword(String, String),
    ApiKey(String),
}


/// Find first matching regular expression and evaluated corresponding expression
macro_rules! match_re {
    ($url:expr, $($pattern:expr => $e:expr),+) => (
        $(
            let re = regex::Regex::new($pattern).unwrap();
            if re.is_match($url) {
                return Ok(Box::new($e));
            }
        )+
)
}

/// Create a corresponding `Source` trait object from url
pub fn source_from_url(url: &str) -> Result<Box<dyn Source>> {
    match_re!(url,
        "flipp.dk" => flipp::Flipp,
        "webtoons.com" => webtoon::Webtoon,
        "universe.leagueoflegends.com" => leagueoflegends::LeagueOfLegends,
        "mangaplus.shueisha.co.jp" => mangaplus::MangaPlus,
        "marvel.com" => marvel::Marvel
    );
    Err(Error::UrlNotSupported(url.to_string()))
}

/// Create source object from name
pub fn source_from_name(name: &str) -> Result<Box<dyn Source>> {
    let lower = name.to_lowercase();
    Ok(match lower.as_str() {
        "flipp" => Box::new(flipp::Flipp),
        "webtoon" => Box::new(webtoon::Webtoon),
        "league of legends" => Box::new(leagueoflegends::LeagueOfLegends),
        "manga plus" => Box::new(mangaplus::MangaPlus),
        "marvel" => Box::new(marvel::Marvel),
        _ => return Err(Error::InvalidSourceName(name.to_string()))
    })
}

/// Trait for interacting with comic book source
/// Trait object can be created with `source_from_url` function
#[async_trait::async_trait]
pub trait Source {
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
    fn get_correct_id(&self, client: &Client, otherid: &ComicId) -> Result<Request<ComicId>> {
        unreachable!()
    }

    /// Retrieves `ComicId` for all comics in series
    /// `seriesid` has to be a `ComicId::Series`
    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<Request<Vec<ComicId>>>;

    /// Creates `SourceREsponse` to download comic metadata
    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>>;

    /// Creates `SourceResponse` to get metadata about series
    #[allow(unused_variables)]
    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>>;

    /// Downloads pages
    #[allow(unused_variables)]
    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<Request<Vec<Page>>> {
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
