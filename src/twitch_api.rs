use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

static HTTP: LazyLock<Client> = LazyLock::new(Client::new);
const API_BASE: &str = "https://api.twitch.tv/helix";

/// Twitch API error type
#[derive(Debug)]
pub enum TwitchApiError {
    Http(reqwest::Error),
    /// 401 Unauthorized — token is invalid or revoked; caller should refresh or re-auth
    Unauthorized,
    Api { status: u16, message: String },
}

impl From<reqwest::Error> for TwitchApiError {
    fn from(err: reqwest::Error) -> Self {
        TwitchApiError::Http(err)
    }
}

impl std::fmt::Display for TwitchApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwitchApiError::Http(e) => write!(f, "HTTP error: {}", e),
            TwitchApiError::Unauthorized => write!(f, "Unauthorized (token expired or revoked)"),
            TwitchApiError::Api { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
        }
    }
}

impl std::error::Error for TwitchApiError {}

/// Helper function to build authorization headers
fn auth_headers(token: &str, client_id: &str) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    headers.insert("Client-Id", client_id.parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers
}

/// Helper function to handle API responses
async fn handle_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, TwitchApiError> {
    let status = response.status().as_u16();

    if status == 401 {
        return Err(TwitchApiError::Unauthorized);
    }

    let body = response.text().await?;

    if status >= 200 && status < 300 {
        serde_json::from_str(&body).map_err(|_| TwitchApiError::Api {
            status,
            message: "Failed to parse response body".to_string(),
        })
    } else {
        // Try to extract error message from response body
        let error_msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
            .unwrap_or_else(|| body.clone());

        Err(TwitchApiError::Api {
            status,
            message: error_msg,
        })
    }

}

/// Helper for void API responses (no body to deserialize), with 401 detection.
async fn handle_void_response(response: reqwest::Response) -> Result<(), TwitchApiError> {
    let status = response.status().as_u16();
    if status == 401 {
        return Err(TwitchApiError::Unauthorized);
    }
    if status >= 200 && status < 300 {
        return Ok(());
    }
    let body = response.text().await?;
    let error_msg = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
        .unwrap_or_else(|| body);
    Err(TwitchApiError::Api { status, message: error_msg })
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatSettings {
    pub emote_mode: bool,
    pub follower_mode: bool,
    pub follower_mode_duration: Option<u32>,
    pub slow_mode: bool,
    pub slow_mode_wait_time: Option<u32>,
    pub subscriber_mode: bool,
}

#[derive(Debug, Deserialize)]
struct DataResponse<T> {
    data: Vec<T>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClipResponse {
    pub id: String,
    pub edit_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShieldModeResponse {
    pub is_active: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StreamInfo {
    pub viewer_count: u32,
    pub title: String,
    pub game_name: String,
}

// ============================================================================
// Request Bodies
// ============================================================================

#[derive(Debug, Serialize)]
struct SendChatMessageBody {
    broadcaster_id: String,
    sender_id: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct CreateStreamMarkerBody {
    user_id: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct StartCommercialBody {
    broadcaster_id: String,
    length: u32,
}

#[derive(Debug, Serialize)]
struct UpdateShieldModeBody {
    is_active: bool,
}

// ============================================================================
// API Functions
// ============================================================================

/// Get chat settings for the broadcaster
pub async fn get_chat_settings(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
    moderator_id: &str,
) -> Result<ChatSettings, TwitchApiError> {
    let url = format!(
        "{}/chat/settings?broadcaster_id={}&moderator_id={}",
        API_BASE, broadcaster_id, moderator_id
    );

    let response = HTTP
        .get(&url)
        .headers(auth_headers(token, client_id))
        .send()
        .await?;

    let data: DataResponse<ChatSettings> = handle_response(response).await?;
    data.data
        .into_iter()
        .next()
        .ok_or_else(|| TwitchApiError::Api {
            status: 200,
            message: "No chat settings in response".to_string(),
        })
}

/// Update chat settings for the broadcaster
pub async fn patch_chat_settings(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
    moderator_id: &str,
    body: Value,
) -> Result<(), TwitchApiError> {
    let url = format!(
        "{}/chat/settings?broadcaster_id={}&moderator_id={}",
        API_BASE, broadcaster_id, moderator_id
    );

    let response = HTTP
        .patch(&url)
        .headers(auth_headers(token, client_id))
        .json(&body)
        .send()
        .await?;

    handle_void_response(response).await
}

/// Send a message to chat
pub async fn send_chat_message(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
    sender_id: &str,
    message: &str,
) -> Result<(), TwitchApiError> {
    let url = format!("{}/chat/messages", API_BASE);

    let body = SendChatMessageBody {
        broadcaster_id: broadcaster_id.to_string(),
        sender_id: sender_id.to_string(),
        message: message.to_string(),
    };

    let response = HTTP
        .post(&url)
        .headers(auth_headers(token, client_id))
        .json(&body)
        .send()
        .await?;

    handle_void_response(response).await
}

/// Clear chat messages
pub async fn clear_chat(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
    moderator_id: &str,
) -> Result<(), TwitchApiError> {
    let url = format!(
        "{}/chat/messages?broadcaster_id={}&moderator_id={}",
        API_BASE, broadcaster_id, moderator_id
    );

    let response = HTTP
        .delete(&url)
        .headers(auth_headers(token, client_id))
        .send()
        .await?;

    handle_void_response(response).await
}

/// Create a stream marker
pub async fn create_stream_marker(
    token: &str,
    client_id: &str,
    user_id: &str,
) -> Result<(), TwitchApiError> {
    let url = format!("{}/streams/markers", API_BASE);

    let body = CreateStreamMarkerBody {
        user_id: user_id.to_string(),
        description: String::new(),
    };

    let response = HTTP
        .post(&url)
        .headers(auth_headers(token, client_id))
        .json(&body)
        .send()
        .await?;

    handle_void_response(response).await
}

/// Create a clip
pub async fn create_clip(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
) -> Result<ClipResponse, TwitchApiError> {
    let url = format!("{}/clips?broadcaster_id={}", API_BASE, broadcaster_id);

    let response = HTTP
        .post(&url)
        .headers(auth_headers(token, client_id))
        .send()
        .await?;

    let data: DataResponse<ClipResponse> = handle_response(response).await?;
    data.data
        .into_iter()
        .next()
        .ok_or_else(|| TwitchApiError::Api {
            status: 200,
            message: "No clip data in response".to_string(),
        })
}

/// Start a commercial
pub async fn start_commercial(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
    length: u32,
) -> Result<(), TwitchApiError> {
    let url = format!("{}/channels/commercial", API_BASE);

    let body = StartCommercialBody {
        broadcaster_id: broadcaster_id.to_string(),
        length,
    };

    let response = HTTP
        .post(&url)
        .headers(auth_headers(token, client_id))
        .json(&body)
        .send()
        .await?;

    handle_void_response(response).await
}

/// Update shield mode status
pub async fn update_shield_mode(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
    moderator_id: &str,
    is_active: bool,
) -> Result<ShieldModeResponse, TwitchApiError> {
    let url = format!(
        "{}/moderation/shield_mode?broadcaster_id={}&moderator_id={}",
        API_BASE, broadcaster_id, moderator_id
    );

    let body = UpdateShieldModeBody { is_active };

    let response = HTTP
        .put(&url)
        .headers(auth_headers(token, client_id))
        .json(&body)
        .send()
        .await?;

    let data: DataResponse<ShieldModeResponse> = handle_response(response).await?;
    data.data
        .into_iter()
        .next()
        .ok_or_else(|| TwitchApiError::Api {
            status: 200,
            message: "No shield mode data in response".to_string(),
        })
}

/// Get current shield mode status
pub async fn get_shield_mode_status(
    token: &str,
    client_id: &str,
    broadcaster_id: &str,
    moderator_id: &str,
) -> Result<ShieldModeResponse, TwitchApiError> {
    let url = format!(
        "{}/moderation/shield_mode?broadcaster_id={}&moderator_id={}",
        API_BASE, broadcaster_id, moderator_id
    );

    let response = HTTP
        .get(&url)
        .headers(auth_headers(token, client_id))
        .send()
        .await?;

    let data: DataResponse<ShieldModeResponse> = handle_response(response).await?;
    data.data
        .into_iter()
        .next()
        .ok_or_else(|| TwitchApiError::Api {
            status: 200,
            message: "No shield mode data in response".to_string(),
        })
}

/// Get stream info (returns None if not live)
pub async fn get_stream_info(
    token: &str,
    client_id: &str,
    user_id: &str,
) -> Result<Option<StreamInfo>, TwitchApiError> {
    let url = format!("{}/streams?user_id={}", API_BASE, user_id);

    let response = HTTP
        .get(&url)
        .headers(auth_headers(token, client_id))
        .send()
        .await?;

    let data: DataResponse<StreamInfo> = handle_response(response).await?;
    Ok(data.data.into_iter().next())
}
