use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use tokio::sync::RwLock;

pub const BUTTON_LABEL_MAX: usize = 10;

pub static TITLE_CACHE: LazyLock<RwLock<std::collections::HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(std::collections::HashMap::new()));

pub async fn cache_title(instance_id: &str, title: &str) {
    TITLE_CACHE.write().await.insert(instance_id.to_string(), title.to_string());
}

pub async fn get_cached_title(instance_id: &str) -> Option<String> {
    TITLE_CACHE.read().await.get(instance_id).cloned()
}

/// Global plugin settings persisted in OpenDeck
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GlobalSettings {
    /// Twitch application Client ID (user must register their own app)
    pub client_id: String,
    /// Twitch application Client Secret
    pub client_secret: String,
    /// OAuth2 access token
    pub access_token: Option<String>,
    /// OAuth2 refresh token (one-time use)
    pub refresh_token: Option<String>,
    /// Unix timestamp when access_token expires
    pub token_expires_at: Option<i64>,
    /// Broadcaster's Twitch user ID
    pub user_id: Option<String>,
    /// Broadcaster's Twitch username (for display)
    pub username: Option<String>,
}

impl GlobalSettings {
    /// Check if the plugin is authenticated with Twitch
    pub fn is_authenticated(&self) -> bool {
        self.access_token.is_some()
            && self.user_id.is_some()
            && !self.client_id.is_empty()
    }
}

/// In-memory global settings store
pub static SETTINGS: LazyLock<RwLock<GlobalSettings>> =
    LazyLock::new(|| RwLock::new(GlobalSettings::default()));

/// Update in-memory settings and persist to OpenDeck
pub async fn save_settings(settings: GlobalSettings) -> openaction::OpenActionResult<()> {
    *SETTINGS.write().await = settings.clone();
    openaction::set_global_settings(&settings).await
}

/// Read a clone of the current global settings
pub async fn read_settings() -> GlobalSettings {
    SETTINGS.read().await.clone()
}

/// Settings for Send Chat Message action
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatMessageSettings {
    pub message: String,
    #[serde(default)]
    pub button_label: Option<String>,
    #[serde(default)]
    pub button_image: Option<String>,
}

fn default_duration_seconds() -> u32 { 30 }
fn default_wait_seconds() -> u32 { 30 }
fn default_follow_duration_minutes() -> u32 { 10 }

/// Settings for Play Ad action
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayAdSettings {
    #[serde(default = "default_duration_seconds")]
    pub duration_seconds: u32,
    #[serde(default)]
    pub button_label: Option<String>,
    #[serde(default)]
    pub button_image: Option<String>,
}

impl Default for PlayAdSettings {
    fn default() -> Self {
        Self { duration_seconds: 30, button_label: None, button_image: None }
    }
}

/// Settings for Slow Chat action
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SlowChatSettings {
    #[serde(default = "default_wait_seconds")]
    pub wait_seconds: u32,
    #[serde(default)]
    pub button_label: Option<String>,
    #[serde(default)]
    pub button_image: Option<String>,
}

impl Default for SlowChatSettings {
    fn default() -> Self {
        Self { wait_seconds: 30, button_label: None, button_image: None }
    }
}

/// Settings for Follower-Only Chat action
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FollowerChatSettings {
    #[serde(default = "default_follow_duration_minutes")]
    pub follow_duration_minutes: u32,
    #[serde(default)]
    pub button_label: Option<String>,
    #[serde(default)]
    pub button_image: Option<String>,
}

impl Default for FollowerChatSettings {
    fn default() -> Self {
        Self { follow_duration_minutes: 10, button_label: None, button_image: None }
    }
}

/// Settings for actions with no configuration
/// Used by: clear_chat, emote_chat, sub_chat, shield_mode, marker, clip, viewers
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EmptySettings {
    #[serde(default)]
    pub button_label: Option<String>,
    #[serde(default)]
    pub button_image: Option<String>,
}
