use super::AccountData;
use ql_core::{IntoJsonError, RequestError, CLIENT};
use serde::Deserialize;

mod error;
pub use error::{AccountError, AccountResponseError};

// Well, no one's gonna be stealing this one :)
pub const CLIENT_ID: &str = "quantumlauncher1";

pub async fn login_fresh(username: String, password: String) -> Result<Account, AccountError> {
    let response = CLIENT
        .post("https://authserver.ely.by/auth/authenticate")
        .json(&serde_json::json!({
            "username": &username,
            "password": &password,
            "clientToken": CLIENT_ID
        }))
        .send()
        .await?;

    let text = if response.status().is_success() {
        response.text().await?
    } else {
        return Err(RequestError::DownloadError {
            code: response.status(),
            url: response.url().clone(),
        }
        .into());
    };

    let account_response = match serde_json::from_str::<AccountResponse>(&text).json(text.clone()) {
        Ok(n) => n,
        Err(err) => {
            if let Ok(res_err) = serde_json::from_str::<AccountResponseError>(&text).json(text) {
                if res_err.error == "ForbiddenOperationException"
                    && res_err.errorMessage == "Account protected with two factor auth."
                {
                    return Ok(Account::NeedsOTP);
                } else {
                    return Err(err.into());
                }
            } else {
                return Err(err.into());
            }
        }
    };

    let entry = keyring::Entry::new("QuantumLauncher", &format!("{username}#elyby"))?;
    entry.set_password(&account_response.accessToken)?;

    Ok(Account::Account(AccountData {
        access_token: Some(account_response.accessToken.clone()),
        uuid: account_response.selectedProfile.id,
        username: account_response.selectedProfile.name,
        refresh_token: account_response.accessToken,
        needs_refresh: false,
        account_type: super::AccountType::ElyBy,
    }))
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
struct AccountResponse {
    pub accessToken: String,
    pub selectedProfile: AccountResponseProfile,
}

#[derive(Deserialize, Clone, Debug)]
struct AccountResponseProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum Account {
    Account(AccountData),
    NeedsOTP,
}
