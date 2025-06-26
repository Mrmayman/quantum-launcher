//! # Minecraft Authentication
//!
//! This module allows you to log into Minecraft with
//! your paid Microsoft account.
//!
//! Taken from <https://github.com/minecraft-rs/auth>
//!
//! ## Modifications:
//! - Changed to `reqwest::Client` and `async`
//!   from `reqwest::blocking::Client`
//! - Changed error handling code
//! - Split it up into clean, independent functions
//!
//! # Login Process
//! ## 1) Adding a new account
//! If you are logging in and adding a new account, then:
//!
//! ```no_run
//! # async fn do1() -> Result<(), Box<dyn std::error::Error>> {
//! use ql_instances::login_1_link;
//! let auth_code_response = login_1_link().await?;
//! // AuthCodeResponse { verification_uri, user_code, .. }
//! # Ok(()) }
//! ```
//!
//! Now we wait for user to open the `verification_uri` link in browser,
//! login with their account,
//! then enter `user_code`.
//!
//! ```no_run
//! # async fn do2() -> Result<(), Box<dyn std::error::Error>> {
//! # // Default construction
//! # let auth_code_response = ql_instances::AuthCodeResponse {
//! #     user_code: String::new(),
//! #     device_code: String::new(),
//! #     verification_uri: String::new(),
//! #     expires_in: 0,
//! #     interval: 0,
//! #     message: String::new(),
//! # };
//! use ql_instances::login_3_xbox;
//! use ql_instances::login_2_wait;
//!
//! let auth_token_response = login_2_wait(auth_code_response).await?;
//! // AuthTokenResponse { access_token, refresh_token }
//!
//! let account_data = login_3_xbox(auth_token_response, None, true).await?;
//! // AccountData { access_token, uuid, username, refresh_token, needs_refresh }
//! # Ok(()) }
//! ```
//!
//! Now save the `username` and corresponding `refresh_token` to disk
//! and play the game with `access_token`.
//!
//! ## 2) Refreshing the account on every play session
//! After starting the launcher later, to refresh
//! the token, we do
//!
//! ```no_run
//! # async fn do3() -> Result<(), Box<dyn std::error::Error>> {
//! # let username = String::new();
//! # let refresh_token = String::new();
//! use ql_instances::login_refresh;
//! let account_data = login_refresh(username, refresh_token, None).await?;
//! # Ok(()) }
//! ```

use ql_core::{
    err, info, pt, retry, GenericProgress, IntoJsonError, IntoStringError, JsonError, RequestError,
    CLIENT,
};
use ql_reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;

/// The API key for logging into Minecraft.
///
/// It's (kinda) safe to leave this public,
/// as the worst that can happen is someone
/// uses this for auth in their own launcher.
/// If you're working on Quantum Launcher or
/// just playing around with your own code
/// **for testing purposes** feel free to use this.
///
/// **Do not use this for any real projects or production code,
/// outside of this launcher**.
pub const CLIENT_ID: &str = "43431a16-38f5-4b42-91f9-4bf70c3bee1e";

#[derive(Debug, Clone)]
pub struct AccountData {
    pub access_token: Option<String>,
    pub uuid: String,
    pub username: String,
    pub refresh_token: String,
    pub needs_refresh: bool,

    pub account_type: AccountType,
}

#[derive(Debug, Clone, Copy)]
pub enum AccountType {
    Microsoft,
    ElyBy,
}

impl AccountData {
    #[must_use]
    pub fn is_elyby(&self) -> bool {
        let account_type = self.account_type;
        matches!(account_type, AccountType::ElyBy)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: isize,
    pub interval: u64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthTokenResponse {
    // pub token_type: String,
    // pub scope: String,
    // pub expires_in: i64,
    // pub ext_expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxLiveAuthResponse {
    issue_instant: String,
    not_after: String,
    token: String,
    display_claims: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Deserialize, Debug, Clone)]
struct MinecraftAuthResponse {
    access_token: String,
    // username: String,
    // roles: Vec<String>,
    // expires_in: u32,
    // token_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RefreshResponse {
    // pub expires_in: u64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct AuthServiceErrorMessage {
    error: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct MinecraftFinalDetails {
    id: Option<String>,
    name: String,
}

const AUTH_ERR_PREFIX: &str = "while managing microsoft account:\n";

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{AUTH_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{AUTH_ERR_PREFIX}Invalid account access token!")]
    InvalidAccessToken,
    #[error("{AUTH_ERR_PREFIX}An unknown error has occured (code: {0})\n\nThis is a major bug! Please report in discord.")]
    UnknownError(StatusCode),
    #[error("{AUTH_ERR_PREFIX}missing JSON field: {0}")]
    MissingField(String),
    #[error("{AUTH_ERR_PREFIX}no uuid found for account")]
    NoUuid,
    #[error("Your microsoft account doesn't own minecraft!\nJust enter the username in the text box instead of logging in.")]
    DoesntOwnGame,

    #[cfg(not(target_os = "linux"))]
    #[error("{AUTH_ERR_PREFIX}keyring error: {0}")]
    KeyringError(#[from] keyring::Error),
    #[cfg(target_os = "linux")]
    #[error("{AUTH_ERR_PREFIX}keyring error: {0}\n\nSee https://mrmayman.github.io/quantumlauncher/#keyring-error for help")]
    KeyringError(#[from] keyring::Error),
}

impl From<ql_reqwest::Error> for AuthError {
    fn from(value: ql_reqwest::Error) -> Self {
        Self::Request(RequestError::ReqwestError(value))
    }
}

pub fn logout(username: &str) -> Result<(), String> {
    let entry = keyring::Entry::new("QuantumLauncher", username).strerr()?;
    if let Err(err) = entry.delete_credential() {
        err!("Couldn't remove account credential: {err}");
    }
    Ok(())
}

/// Gets the account info from the
/// refresh token.
///
/// You can read an existing refresh token
/// from disk using [`read_refresh_token`].
///
/// Note: This is for reusing an existing logged-in
/// account. If you want to freshly log in, use
/// [`login_1_link`], [`login_2_wait`], [`login_3_xbox`]
/// respectively in that order.
pub async fn login_refresh(
    username: String,
    refresh_token: String,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
) -> Result<AccountData, AuthError> {
    send_progress(sender.as_ref(), 0, 4, "Refreshing account token...");

    let response = retry(async || {
        CLIENT
            .post("https://login.live.com/oauth20_token.srf")
            .form(&[
                ("client_id", CLIENT_ID),
                ("refresh_token", &refresh_token),
                ("grant_type", "refresh_token"),
                ("redirect_uri", "https://login.live.com/oauth20_desktop.srf"),
                ("scope", "XboxLive.signin offline_access"),
            ])
            .send()
            .await?
            .text()
            .await
    })
    .await?;

    let data: RefreshResponse = serde_json::from_str(&response).json(response)?;

    let entry = keyring::Entry::new("QuantumLauncher", &username)?;
    entry.set_password(&data.refresh_token)?;

    let data = login_3_xbox(
        AuthTokenResponse {
            access_token: data.access_token,
            refresh_token: data.refresh_token,
        },
        sender,
        false,
    )
    .await?;

    Ok(data)
}

pub async fn login_1_link() -> Result<AuthCodeResponse, AuthError> {
    info!("Logging into Microsoft Account...");

    pt!("Sending device code request");
    let response = CLIENT
        .get("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .query(&[
            ("client_id", CLIENT_ID),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?
        .text()
        .await?;

    let data: AuthCodeResponse = serde_json::from_str(&response).json(response)?;

    pt!(
        "Go to {} and enter code {} (shown in menu)",
        data.verification_uri,
        data.user_code
    );

    Ok(data)
}

pub fn read_refresh_token(username: &str) -> Result<String, AuthError> {
    let entry = keyring::Entry::new("QuantumLauncher", username)?;
    let refresh_token = entry.get_password()?;
    Ok(refresh_token)
}

pub async fn login_3_xbox(
    data: AuthTokenResponse,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
    check_ownership: bool,
) -> Result<AccountData, AuthError> {
    let steps = if check_ownership { 5 } else { 4 };

    send_progress(sender.as_ref(), 1, steps, "Logging into xbox live...");
    let xbox = login_in_xbox_live(&CLIENT, &data).await?;
    send_progress(sender.as_ref(), 2, steps, "Logging into minecraft...");
    let minecraft = login_in_minecraft(&CLIENT, &xbox).await?;
    send_progress(sender.as_ref(), 3, steps, "Getting account details...");
    let final_details = get_final_details(&CLIENT, &minecraft).await?;

    if check_ownership {
        send_progress(sender.as_ref(), 4, steps, "Checking game ownership...");
        let owns_game = check_minecraft_ownership(&minecraft.access_token).await?;

        if !owns_game {
            return Err(AuthError::DoesntOwnGame);
        }
    }

    let entry = keyring::Entry::new("QuantumLauncher", &final_details.name)?;
    entry.set_password(&data.refresh_token)?;

    let data = AccountData {
        access_token: Some(minecraft.access_token),
        uuid: final_details.id.ok_or(AuthError::NoUuid)?,
        username: final_details.name,
        refresh_token: data.refresh_token,
        needs_refresh: false,
        account_type: AccountType::Microsoft,
    };

    info!("Finished Microsoft Account login!");

    Ok(data)
}

fn send_progress(
    sender: Option<&std::sync::mpsc::Sender<GenericProgress>>,
    done: usize,
    total: usize,
    message: &str,
) {
    pt!("{message}");
    if let Some(sender) = sender {
        _ = sender.send(GenericProgress {
            done,
            total,
            message: Some(message.to_owned()),
            has_finished: false,
        });
    }
}

pub async fn login_2_wait(response: AuthCodeResponse) -> Result<AuthTokenResponse, AuthError> {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(response.interval + 1)).await;

        let code_resp = CLIENT
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&[
                ("client_id", CLIENT_ID),
                ("scope", "XboxLive.signin offline_access"),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &response.device_code),
            ])
            .send()
            .await?;

        match code_resp.status() {
            StatusCode::BAD_REQUEST => {
                let txt = code_resp.text().await?;
                let error: AuthServiceErrorMessage = serde_json::from_str(&txt).json(txt)?;
                match &error.error as &str {
                    "authorization_declined" | "expired_token" | "invalid_grant" => {
                        return Err(AuthError::InvalidAccessToken);
                    }
                    _ => {}
                }
            }

            StatusCode::OK => {
                let text = code_resp.text().await?;
                let response: AuthTokenResponse = serde_json::from_str(&text).json(text)?;
                return Ok(response);
            }
            code => {
                return Err(AuthError::UnknownError(code));
            }
        }
    }
}

async fn login_in_xbox_live(
    client: &Client,
    auth_token: &AuthTokenResponse,
) -> Result<XboxLiveAuthResponse, AuthError> {
    let xbox_authenticate_json = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": &format!("d={}", auth_token.access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbox_res = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&xbox_authenticate_json)
        .send()
        .await?
        .text()
        .await?;

    let xbox_res: XboxLiveAuthResponse = serde_json::from_str(&xbox_res).json(xbox_res)?;
    Ok(xbox_res)
}

async fn login_in_minecraft(
    client: &Client,
    xbox_res: &XboxLiveAuthResponse,
) -> Result<MinecraftAuthResponse, AuthError> {
    let xbox_token = &xbox_res.token;
    let user_hash = &xbox_res
        .display_claims
        .get("xui")
        .ok_or(AuthError::MissingField(
            "xbox_res.display_claims.xui".to_owned(),
        ))?
        .first()
        .ok_or(AuthError::MissingField(
            "xbox_res.display_claims.xui[0]".to_owned(),
        ))?
        .get("uhs")
        .ok_or(AuthError::MissingField(
            "xbox_res.display_claims.xui[0].uhs".to_owned(),
        ))?;

    let xbox_security_token_res = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbox_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .text()
        .await?;

    let xbox_security_token_res: XboxLiveAuthResponse =
        serde_json::from_str(&xbox_security_token_res).json(xbox_security_token_res)?;

    let xbox_security_token = &xbox_security_token_res.token;

    let minecraft_resp = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({
            "identityToken":
                format!(
                    "XBL3.0 x={user_hash};{xbox_security_token}"
                )
        }))
        .send()
        .await?
        .text()
        .await?;

    let minecraft_resp: MinecraftAuthResponse =
        serde_json::from_str(&minecraft_resp).json(minecraft_resp)?;
    Ok(minecraft_resp)
}

async fn get_final_details(
    client: &Client,
    minecraft_res: &MinecraftAuthResponse,
) -> Result<MinecraftFinalDetails, AuthError> {
    let text = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Accept", "application/json")
        .bearer_auth(&minecraft_res.access_token)
        .send()
        .await?
        .text()
        .await?;

    Ok(serde_json::from_str(&text).json(text)?)
}

async fn check_minecraft_ownership(access_token: &str) -> Result<bool, AuthError> {
    #[derive(Deserialize)]
    struct Ownership {
        items: Vec<serde_json::Value>,
    }

    let client = Client::new();

    let response = client
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(access_token)
        .send()
        .await?
        .text()
        .await?;
    let response: Ownership = serde_json::from_str(&response).json(response)?;

    Ok(!response.items.is_empty())
}
