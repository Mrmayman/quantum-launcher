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

use ql_core::RequestError;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

// Please don't steal :)
pub const CLIENT_ID: &str = "43431a16-38f5-4b42-91f9-4bf70c3bee1e";

pub struct AccountData {
    pub access_token: String,
    pub uuid: String,
    pub username: String,
    pub refresh_token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: u64,
    pub message: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthTokenResponse {
    pub token_type: String,
    pub scope: String,
    pub expires_in: i64,
    pub ext_expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxLiveAuthResponse {
    issue_instant: String,
    not_after: String,
    token: String,
    display_claims: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct MinecraftAuthResponse {
    username: String,
    roles: Vec<String>,
    access_token: String,
    expires_in: u32,
    token_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AuthServiceErrorMessage {
    error: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MinecraftFinalDetails {
    id: Option<String>,
    name: String,
}

#[derive(Debug)]
pub enum AuthError {
    RequestError(RequestError),
    SerdeError(serde_json::Error, String),
    InvalidAccessToken,
    UnknownError,
    MissingField(String),
    NoUuid,
}

impl std::error::Error for AuthError {}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error authenticating with microsoft: ")?;
        match self {
            AuthError::RequestError(err) => write!(f, "{err}"),
            AuthError::SerdeError(err, json) => write!(f, "(json) {err}\njson: {json}"),
            AuthError::InvalidAccessToken => write!(f, "invalid access token"),
            AuthError::UnknownError => write!(f, "unknown error"),
            AuthError::MissingField(err) => write!(f, "missing field: {err}"),
            AuthError::NoUuid => write!(f, "no uuid found"),
        }
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(value: reqwest::Error) -> Self {
        Self::RequestError(RequestError::ReqwestError(value))
    }
}

impl From<RequestError> for AuthError {
    fn from(value: RequestError) -> Self {
        Self::RequestError(value)
    }
}

pub async fn login_3_xbox_w(data: AuthTokenResponse) -> Result<AccountData, String> {
    login_3_xbox(data).await.map_err(|err| err.to_string())
}

pub async fn login_1_link_w() -> Result<AuthCodeResponse, String> {
    login_1_link().await.map_err(|err| err.to_string())
}

pub async fn login_1_link() -> Result<AuthCodeResponse, AuthError> {
    let client = reqwest::Client::new();

    let response = client
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
    Ok(data)
}

pub async fn login_2_wait_w(data: AuthCodeResponse) -> Result<AuthTokenResponse, String> {
    login_2_wait(data).await.map_err(|err| err.to_string())
}

pub async fn login_2_wait(data: AuthCodeResponse) -> Result<AuthTokenResponse, AuthError> {
    let client = reqwest::Client::new();
    let token = wait_for_login(&client, &data).await?;
    Ok(token)
}

pub async fn login_3_xbox(data: AuthTokenResponse) -> Result<AccountData, AuthError> {
    let client = reqwest::Client::new();
    let xbox = login_in_xbox_live(&client, &data).await?;
    let minecraft = login_in_minecraft(&client, &xbox).await?;
    let final_details = get_final_details(&client, &minecraft).await?;

    let data = AccountData {
        access_token: minecraft.access_token,
        uuid: final_details.id.ok_or(AuthError::NoUuid)?,
        username: final_details.name,
        refresh_token: data.refresh_token,
    };

    Ok(data)
}

async fn wait_for_login(
    client: &Client,
    response: &AuthCodeResponse,
) -> Result<AuthTokenResponse, AuthError> {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(response.interval + 1)).await;

        let code_resp = client
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
                    "authorization_declined" => {
                        return Err(AuthError::InvalidAccessToken);
                    }
                    "expired_token" => {
                        return Err(AuthError::InvalidAccessToken);
                    }
                    "invalid_grant" => {
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
                    "XBL3.0 x={user_hash};{xsts_token}",
                    user_hash = user_hash,
                    xsts_token = xbox_security_token
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
