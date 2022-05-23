mod download;
mod flipp;

use crate::{
    error::GrawlixDownloadError as Error,
    metadata::Metadata,
    comic::Page
};
use reqwest::Client;
pub use download::{download_comics, download_comics_metadata};

/// Result type with `GrawlixDownloadError`
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
/// Id of comic or series on source
pub enum ComicId {
    Issue(String),
    IssueWithMetadata(String, Metadata),
    Other(String),
    OtherWithMetadata(String, Metadata),
    Series(String),
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
        "flipp.dk" => flipp::Flipp
    );
    Err(Error::UrlNotSupported(url.to_string()))
}

/// Trait for interacting with comic book source
/// Trait object can be created with `source_from_url` function
pub trait Source {
    /// Name of source
    fn name(&self) -> String;

    /// Create `reqwest::Client` to use for all requests generated from source
    fn create_client(&self) -> reqwest::Client {
        reqwest::Client::new()
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

    /// Creates `Request` to download comic metadata
    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>>;

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
    fn authenticate(&mut self, client: &mut Client, creds: &Credentials) -> Result<()> {
        Ok(())
    }

}

/// Converts binary response to json
fn resp_to_json<'a, T: serde::Deserialize<'a>>(response: &'a [u8]) -> Option<T> {
    serde_json::from_str(std::str::from_utf8(response).ok()?).ok()
}

/// Converts `serde_json::Value` to `Option<String>`
fn value_to_optstring(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(|x| x.to_string())
}

/// Find first matching capture in regex
fn first_capture(re: regex::Regex, text: &str) -> Option<String> {
    Some(re.captures(text)?.get(1)?.as_str().to_string())
}
