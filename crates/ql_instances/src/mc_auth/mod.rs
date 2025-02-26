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

use ql_core::{info, pt, GenericProgress, RequestError, CLIENT};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;

// Please don't steal :)
pub const CLIENT_ID: &str = "43431a16-38f5-4b42-91f9-4bf70c3bee1e";

#[derive(Debug, Clone)]
pub struct AccountData {
    pub access_token: Option<String>,
    pub uuid: String,
    pub username: String,
    pub refresh_token: String,
    pub needs_refresh: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct AuthCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: u64,
    pub message: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct AuthTokenResponse {
    // pub token_type: String,
    // pub scope: String,
    // pub expires_in: i64,
    // pub ext_expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
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

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("microsoft account error: {0}")]
    RequestError(#[from] RequestError),
    #[error("microsoft account error: json error: {0}\njson: {1}")]
    SerdeError(serde_json::Error, String),
    #[error("microsoft account error: invalid access token")]
    InvalidAccessToken,
    #[error("microsoft account error: unknown error")]
    UnknownError,
    #[error("microsoft account error: missing json field: {0}")]
    MissingField(String),
    #[error("microsoft account error: no uuid found")]
    NoUuid,
    #[error("microsoft account doesn't own minecraft")]
    DoesntOwnGame,
    #[error("microsoft account error: keyring error: {0}")]
    KeyringError(#[from] keyring::Error),
}

impl From<reqwest::Error> for AuthError {
    fn from(value: reqwest::Error) -> Self {
        Self::RequestError(RequestError::ReqwestError(value))
    }
}

pub async fn login_refresh_w(
    username: String,
    refresh_token: String,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
) -> Result<AccountData, String> {
    login_refresh(&username, &refresh_token, sender)
        .await
        .map_err(|err| err.to_string())
}

pub async fn login_refresh(
    username: &str,
    refresh_token: &str,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
) -> Result<AccountData, AuthError> {
    info!("Refreshing account token...");
    send_progress(sender.as_ref(), 0, "Refreshing account token...");

    let response = CLIENT
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("redirect_uri", "https://login.live.com/oauth20_desktop.srf"),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?
        .text()
        .await?;

    let data: RefreshResponse =
        serde_json::from_str(&response).map_err(|n| AuthError::SerdeError(n, response))?;

    let entry = keyring::Entry::new("QuantumLauncher", username)?;
    entry.set_password(&data.refresh_token)?;

    let data = login_3_xbox(
        AuthTokenResponse {
            access_token: data.access_token,
            refresh_token: data.refresh_token,
        },
        sender,
    )
    .await?;

    Ok(data)
}

pub async fn login_3_xbox_w(
    data: AuthTokenResponse,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
) -> Result<AccountData, String> {
    login_3_xbox(data, sender)
        .await
        .map_err(|err| err.to_string())
}

pub async fn login_1_link_w() -> Result<AuthCodeResponse, String> {
    login_1_link().await.map_err(|err| err.to_string())
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

    let data: AuthCodeResponse =
        serde_json::from_str(&response).map_err(|n| AuthError::SerdeError(n, response))?;

    pt!(
        "Go to {} and enter code {} (shown in menu)",
        data.verification_uri,
        data.user_code
    );

    Ok(data)
}

pub async fn login_2_wait_w(data: AuthCodeResponse) -> Result<AuthTokenResponse, String> {
    login_2_wait(&data).await.map_err(|err| err.to_string())
}

pub fn read_refresh_token(username: &str) -> Result<String, AuthError> {
    let entry = keyring::Entry::new("QuantumLauncher", username)?;
    let refresh_token = entry.get_password()?;
    Ok(refresh_token)
}

pub async fn login_3_xbox(
    data: AuthTokenResponse,
    sender: Option<std::sync::mpsc::Sender<GenericProgress>>,
) -> Result<AccountData, AuthError> {
    send_progress(sender.as_ref(), 0, "Logging into xbox live...");
    let xbox = login_in_xbox_live(&CLIENT, &data).await?;
    send_progress(sender.as_ref(), 1, "Logging into minecraft...");
    let minecraft = login_in_minecraft(&CLIENT, &xbox).await?;
    send_progress(sender.as_ref(), 2, "Getting account details...");
    let final_details = get_final_details(&CLIENT, &minecraft).await?;
    // send_progress(sender.as_ref(), 3, "Checking game ownership...");
    // let owns_game = check_minecraft_ownership(&minecraft.access_token).await?;

    // if !owns_game {
    //     return Err(AuthError::DoesntOwnGame);
    // }

    let entry = keyring::Entry::new("QuantumLauncher", &final_details.name)?;
    entry.set_password(&data.refresh_token)?;

    let data = AccountData {
        access_token: Some(minecraft.access_token),
        uuid: final_details.id.ok_or(AuthError::NoUuid)?,
        username: final_details.name,
        refresh_token: data.refresh_token,
        needs_refresh: false,
    };

    Ok(data)
}

fn send_progress(
    sender: Option<&std::sync::mpsc::Sender<GenericProgress>>,
    done: usize,
    message: &str,
) {
    pt!("{message}");
    if let Some(sender) = sender {
        _ = sender.send(GenericProgress {
            done,
            total: 3,
            message: Some(message.to_owned()),
            has_finished: false,
        });
    }
}

pub async fn login_2_wait(response: &AuthCodeResponse) -> Result<AuthTokenResponse, AuthError> {
    // This code is ugly but it's not my code :')
    // If the top comment wasn't clear enough this was taken from
    // https://github.com/minecraft-rs/auth
    //
    // Massive props to them for even implementing something,
    // that makes them already ahead of me.
    //
    // (WTF: ) tag so people can search for it
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
                let error: AuthServiceErrorMessage =
                    serde_json::from_str(&txt).map_err(|n| AuthError::SerdeError(n, txt))?;
                match &error.error as &str {
                    "authorization_declined" | "expired_token" | "invalid_grant" => {
                        return Err(AuthError::InvalidAccessToken);
                    }
                    _ => {
                        continue;
                    }
                }
            }

            StatusCode::OK => {
                let text = code_resp.text().await?;
                let response: AuthTokenResponse =
                    serde_json::from_str(&text).map_err(|n| AuthError::SerdeError(n, text))?;
                return Ok(response);
            }
            _ => {
                return Err(AuthError::UnknownError);
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

    let xbox_res: XboxLiveAuthResponse =
        serde_json::from_str(&xbox_res).map_err(|n| AuthError::SerdeError(n, xbox_res))?;
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

    let xbox_security_token_res: XboxLiveAuthResponse = client
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
        .json()
        .await?;

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

    let minecraft_resp: MinecraftAuthResponse = serde_json::from_str(&minecraft_resp)
        .map_err(|n| AuthError::SerdeError(n, minecraft_resp))?;
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

    serde_json::from_str(&text).map_err(|n| AuthError::SerdeError(n, text))
}

// async fn check_minecraft_ownership(access_token: &str) -> Result<bool, AuthError> {
//     #[derive(Deserialize)]
//     struct Ownership {
//         items: Vec<Value>,
//     }

//     let client = Client::new();

//     let response = client
//         .get("https://api.minecraftservices.com/entitlements/mcstore")
//         .bearer_auth(access_token)
//         .send()
//         .await?
//         .json::<Ownership>() // Deserialize response as JSON
//         .await?;

//     Ok(!response.items.is_empty())
// }
