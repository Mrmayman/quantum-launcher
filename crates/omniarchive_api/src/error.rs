use ql_core::{JsonDownloadError, RequestError};
use thiserror::Error;

const WEB_SCRAPE_PREFIX: &str =
    "while loading list of downloadable versions from Omniarchive website";

#[derive(Debug, Error)]
pub enum WebScrapeError {
    #[error("{WEB_SCRAPE_PREFIX}:\n{0}")]
    RequestError(#[from] RequestError),
    #[error("{WEB_SCRAPE_PREFIX}: element not found: {0}")]
    ElementNotFound(String),
}

#[derive(Debug, Error)]
pub enum ListError {
    #[error("while loading list of downloadable versions:\n{0}")]
    JsonDownloadError(#[from] JsonDownloadError),
    #[error(transparent)]
    WebScrapeError(#[from] WebScrapeError),
}
