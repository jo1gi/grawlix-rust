use thiserror::Error;

#[derive(Debug, Error)]
/// Grawlix standard error
pub enum GrawlixError {
    #[error("Failed to write comic to disk")]
    Write(#[from] GrawlixIOError),
    #[error(transparent)]
    Download(#[from] GrawlixDownloadError),
}

#[derive(Debug, Error)]
/// Error for write related problems
pub enum GrawlixIOError {
    #[error("Failed to export metadata in {0} format")]
    MetadataExport(String),
    #[error("Failed to import metadata in {0} format")]
    MetadataImport(String),
    #[error("The output location {0} is not valid")]
    InvalidLocation(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error("Could not format comic. Error at index {0} in template: {1}")]
    StringFormat(usize, String),
    #[error("Could not recognize filetype of {0}")]
    UnknownFileType(String),
}

#[derive(Debug, Error)]
/// Error for download related problems
pub enum GrawlixDownloadError {
    #[error("Downloading pages of comic book is not supported on {0}")]
    PagesNotSupported(String),
    #[error("Failed to authenticate with {0}")]
    FailedAuthentication(String),
    #[error("Failed to download from {0}")]
    FailedDownload(String),
    #[error("Failed to make request")]
    RequestError(#[from] reqwest::Error),
    #[error("Url not supported: {0}")]
    UrlNotSupported(String),
    #[error("Failed to parse response")]
    FailedResponseParse,
}
