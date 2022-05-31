use thiserror::Error;
use displaydoc::Display;

#[derive(Debug, Error, Display)]
/// Grawlix standard error
pub enum GrawlixError {
    /// {0}
    Write(#[from] GrawlixIOError),
    /// {0}
    Download(#[from] GrawlixDownloadError),
}

#[derive(Debug, Error, Display)]
/// Error for write related problems
pub enum GrawlixIOError {
    /// Failed to export metadata in {0} format
    MetadataExport(String),
    /// Failed to import metadata in {0} format
    MetadataImport(String),
    /// The output location {0} is not valid
    InvalidLocation(String),
    /// {0}
    Io(#[from] std::io::Error),
    /// {0}
    Zip(#[from] zip::result::ZipError),
    /// Could not format comic. Error at index {0} in template: {1}
    StringFormat(usize, String),
    /// Could not recognize filetype of {0}
    UnknownFileType(String),
}

#[derive(Debug, Error, Display)]
/// Error for download related problems
pub enum GrawlixDownloadError {
    /// Downloading pages of comic book is not supported on {0}
    PagesNotSupported(String),
    /// Failed to authenticate with {0}
    FailedAuthentication(String),
    /// Failed to download from {0}
    FailedDownload(String),
    /// Failed to make request
    RequestError(#[from] reqwest::Error),
    /// Url not supported: {0}
    UrlNotSupported(String),
    /// Failed to parse response
    FailedResponseParse,
}
