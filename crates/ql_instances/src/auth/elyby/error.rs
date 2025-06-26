use ql_core::{JsonError, RequestError};
use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct AccountResponseError {
    pub error: String,
    pub errorMessage: String,
}

impl std::error::Error for AccountResponseError {}
impl std::fmt::Display for AccountResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error, self.errorMessage)
    }
}

const AUTH_ERR_PREFIX: &str = "while logging into ely.by account:\n";
#[derive(Debug, Error)]
pub enum AccountError {
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{AUTH_ERR_PREFIX}\n{0}")]
    Response(#[from] AccountResponseError),

    #[cfg(not(target_os = "linux"))]
    #[error("{AUTH_ERR_PREFIX}keyring error: {0}")]
    KeyringError(#[from] keyring::Error),
    #[cfg(target_os = "linux")]
    #[error("{AUTH_ERR_PREFIX}keyring error: {0}\n\nSee https://mrmayman.github.io/quantumlauncher/#keyring-error for help")]
    KeyringError(#[from] keyring::Error),
}

impl From<ql_reqwest::Error> for AccountError {
    fn from(value: ql_reqwest::Error) -> Self {
        Self::Request(RequestError::ReqwestError(value))
    }
}
