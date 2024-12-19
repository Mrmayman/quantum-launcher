use std::fmt::Display;

use ql_core::RequestError;

#[derive(Debug)]
pub enum WebScrapeError {
    RequestError(RequestError),
    ElementNotFound(String),
}

impl From<RequestError> for WebScrapeError {
    fn from(error: RequestError) -> Self {
        WebScrapeError::RequestError(error)
    }
}

impl Display for WebScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "web scrape error: ")?;
        match self {
            WebScrapeError::RequestError(error) => write!(f, "{}", error),
            WebScrapeError::ElementNotFound(name) => write!(f, "element not found: {}", name),
        }
    }
}
