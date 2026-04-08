use openaction::global_events::{DidReceiveGlobalSettingsEvent, GlobalEventHandler};
use openaction::OpenActionResult;
use openaction::async_trait;
use crate::settings::{GlobalSettings, SETTINGS};

pub struct TwitchGlobalHandler;

#[async_trait]
impl GlobalEventHandler for TwitchGlobalHandler {
    async fn plugin_ready(&self) -> OpenActionResult<()> {
        log::info!("Twitch plugin ready, loading global settings...");
        openaction::get_global_settings().await
    }

    async fn did_receive_global_settings(&self, event: DidReceiveGlobalSettingsEvent) -> OpenActionResult<()> {
        let settings: GlobalSettings = serde_json::from_value(event.payload.settings)
            .unwrap_or_default();
        *SETTINGS.write().await = settings;
        log::info!("Global settings loaded");
        Ok(())
    }
}
