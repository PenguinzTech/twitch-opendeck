use crate::settings::{read_settings, save_settings, GlobalSettings};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

pub const SCOPES: &str = "moderator:manage:chat_settings user:write:chat moderator:manage:chat_messages user:edit:broadcast clips:edit channel:edit:commercial moderator:manage:shield_mode";

#[derive(Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<Vec<String>>,
    pub token_type: Option<String>,
}

#[derive(Deserialize)]
pub struct ValidateResponse {
    pub user_id: String,
    pub login: String,
    pub expires_in: i64,
}

#[derive(Debug)]
pub enum AuthError {
    Http(reqwest::Error),
    Api(String),
    Pending,
    SlowDown,
    Expired,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Http(e) => write!(f, "HTTP error: {}", e),
            AuthError::Api(msg) => write!(f, "API error: {}", msg),
            AuthError::Pending => write!(f, "Authorization pending"),
            AuthError::SlowDown => write!(f, "Slow down"),
            AuthError::Expired => write!(f, "Token expired"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<reqwest::Error> for AuthError {
    fn from(err: reqwest::Error) -> Self {
        AuthError::Http(err)
    }
}

/// Start the device code flow. Returns the response with user_code and verification_uri to display.
pub async fn start_device_flow(client_id: &str) -> Result<DeviceCodeResponse, AuthError> {
    let client = Client::new();
    let params = [("client_id", client_id), ("scopes", SCOPES)];

    let response = client
        .post("https://id.twitch.tv/oauth2/device")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AuthError::Api(format!(
            "Failed to start device flow: {}",
            error_text
        )));
    }

    let device_code_response: DeviceCodeResponse = response.json().await?;
    Ok(device_code_response)
}

/// Poll for a token after starting device flow. Polls every `interval` seconds until authorized,
/// denied, or expired. This is a blocking poll - run in a spawned task.
pub async fn poll_for_token(
    client_id: &str,
    client_secret: &str,
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> Result<TokenResponse, AuthError> {
    let client = Client::new();
    let start_time = Utc::now().timestamp() as u64;
    let mut current_interval = interval;

    loop {
        // Check if we've exceeded the expiration time
        let elapsed = Utc::now().timestamp() as u64 - start_time;
        if elapsed >= expires_in {
            return Err(AuthError::Expired);
        }

        // Wait before polling (except on first iteration)
        if elapsed > 0 {
            sleep(Duration::from_secs(current_interval)).await;
        }

        // Poll for token
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        let response = client
            .post("https://id.twitch.tv/oauth2/token")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: TokenResponse = response.json().await?;
            return Ok(token_response);
        }

        // Parse error response
        let error_body: serde_json::Value = match response.json().await {
            Ok(body) => body,
            Err(_) => {
                return Err(AuthError::Api(
                    "Failed to parse error response".to_string(),
                ))
            }
        };

        // Twitch uses "message" field (not "error") for device code flow responses
        let error_code = error_body
            .get("error")
            .and_then(|e| e.as_str())
            .or_else(|| error_body.get("message").and_then(|m| m.as_str()))
            .unwrap_or("");

        let error_description = error_body
            .get("error_description")
            .and_then(|m| m.as_str())
            .unwrap_or(error_code);

        match error_code {
            "authorization_pending" => continue,
            "slow_down" => { current_interval += 5; continue; }
            "expired_token" | "device_code_expired" => return Err(AuthError::Expired),
            "access_denied" => return Err(AuthError::Api("Access denied by user".to_string())),
            _ => {
                return Err(AuthError::Api(format!(
                    "{} — {}",
                    error_code,
                    error_description
                )));
            }
        }
    }
}

/// Refresh an expired access token.
pub async fn refresh_access_token(client_id: &str, refresh_token: &str) -> Result<TokenResponse, AuthError> {
    let client = Client::new();
    let params = [
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let response = client
        .post("https://id.twitch.tv/oauth2/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AuthError::Api(format!(
            "Failed to refresh token: {}",
            error_text
        )));
    }

    let token_response: TokenResponse = response.json().await?;
    Ok(token_response)
}

/// Validate and get user info from a token.
pub async fn validate_token(access_token: &str) -> Result<ValidateResponse, AuthError> {
    let client = Client::new();
    let response = client
        .get("https://id.twitch.tv/oauth2/validate")
        .header("Authorization", format!("OAuth {}", access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AuthError::Api(format!(
            "Failed to validate token: {}",
            error_text
        )));
    }

    let validate_response: ValidateResponse = response.json().await?;
    Ok(validate_response)
}

/// Get a valid (non-expired) access token, refreshing if needed.
/// Returns (access_token, user_id, client_id) if authenticated, None otherwise.
pub async fn get_valid_token() -> Option<(String, String, String)> {
    let settings = read_settings().await;

    // Check if authenticated
    if !settings.is_authenticated() {
        return None;
    }

    let access_token = settings.access_token.clone()?;
    let user_id = settings.user_id.clone()?;
    let client_id = settings.client_id.clone();

    // Check if token expires within 60 seconds
    let now = Utc::now().timestamp();
    if let Some(expires_at) = settings.token_expires_at {
        if expires_at - now < 60 {
            // Try to refresh
            if let Some(rt) = settings.refresh_token.clone() {
                match refresh_access_token(&client_id, &rt).await {
                    Ok(new_token) => {
                        // Save new token to settings
                        let new_expires_at = Utc::now().timestamp() + new_token.expires_in.unwrap_or(14400);
                        let mut updated = settings.clone();
                        updated.access_token = Some(new_token.access_token.clone());
                        updated.refresh_token = new_token.refresh_token;
                        updated.token_expires_at = Some(new_expires_at);
                        let _ = save_settings(updated).await;
                        return Some((new_token.access_token, user_id, client_id));
                    }
                    Err(_) => return None,
                }
            }
        }
    }

    Some((access_token, user_id, client_id))
}

/// Store a token response into global settings (after successful auth or refresh).
pub async fn store_token(
    token: TokenResponse,
    validate: ValidateResponse,
) -> openaction::OpenActionResult<()> {
    let expires_at = Utc::now().timestamp() + token.expires_in.unwrap_or(14400);
    let mut settings = read_settings().await;
    settings.access_token = Some(token.access_token);
    settings.refresh_token = token.refresh_token;
    settings.token_expires_at = Some(expires_at);
    settings.user_id = Some(validate.user_id);
    settings.username = Some(validate.login);
    save_settings(settings).await
}
