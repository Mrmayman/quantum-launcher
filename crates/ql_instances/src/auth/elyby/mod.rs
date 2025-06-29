use super::AccountData;
use ql_core::{err, info, pt, IntoJsonError, IntoStringError, RequestError, CLIENT};
use serde::Deserialize;

mod authlib;
mod error;
pub(crate) use authlib::get_authlib_injector;
pub use error::{AccountError, AccountResponseError};

// Well, no one's gonna be stealing this one :)
pub const CLIENT_ID: &str = "quantumlauncher1";

pub async fn login_new(username: String, password: String) -> Result<Account, AccountError> {
    info!("Logging into elyby... (username/email: {username})");
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

    let entry = get_keyring_entry(&username)?;
    entry.set_password(&account_response.accessToken)?;

    Ok(Account::Account(AccountData {
        access_token: Some(account_response.accessToken.clone()),
        uuid: account_response.selectedProfile.id,
        username,
        refresh_token: account_response.accessToken,
        needs_refresh: false,
        account_type: super::AccountType::ElyBy,
    }))
}

pub fn read_refresh_token(username: &str) -> Result<String, AccountError> {
    let entry = get_keyring_entry(username)?;
    Ok(entry.get_password()?)
}

pub async fn login_refresh(
    username: String,
    refresh_token: String,
) -> Result<AccountData, AccountError> {
    pt!("Refreshing ely.by account...");
    let entry = get_keyring_entry(&username)?;

    let response = CLIENT
        .post("https://authserver.ely.by/auth/refresh")
        .json(&serde_json::json!({
            "accessToken": refresh_token,
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

    let account_response = serde_json::from_str::<AccountResponse>(&text).json(text.clone())?;
    entry.set_password(&account_response.accessToken)?;

    Ok(AccountData {
        access_token: Some(account_response.accessToken.clone()),
        uuid: account_response.selectedProfile.id,
        username,
        refresh_token: account_response.accessToken,
        needs_refresh: false,
        account_type: super::AccountType::ElyBy,
    })
}

fn get_keyring_entry(username: &str) -> Result<keyring::Entry, AccountError> {
    Ok(keyring::Entry::new(
        "QuantumLauncher",
        &format!("{username}#elyby"),
    )?)
}

pub fn logout(username: &str) -> Result<(), String> {
    let entry = get_keyring_entry(username).strerr()?;
    if let Err(err) = entry.delete_credential() {
        err!("Couldn't remove account credential: {err}");
    }
    Ok(())
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
    // pub name: String,
}

#[derive(Debug, Clone)]
pub enum Account {
    Account(AccountData),
    NeedsOTP,
}
