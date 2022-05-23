pub mod comic;
pub mod error;
pub mod metadata;
pub mod source;

pub use error::GrawlixError as Error;
pub type Result<T> = std::result::Result<T, error::GrawlixError>;
