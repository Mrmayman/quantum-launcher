use ql_core::{JsonDownloadError, RequestError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebScrapeError {
    #[error("could not scrape omniarchive: {0}")]
    RequestError(#[from] RequestError),
    #[error("could not scrape omniarchive: element not found: {0}")]
    ElementNotFound(String),
}

#[derive(Debug, Error)]
pub enum ListError {
    #[error("error listing versions: {0}")]
    JsonDownloadError(#[from] JsonDownloadError),
    #[error(transparent)]
    WebScrapeError(#[from] WebScrapeError),
}
