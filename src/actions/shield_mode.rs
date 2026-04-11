use crate::auth::get_valid_token;
use crate::settings::EmptySettings;
use crate::twitch_api;
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde_json::Value;
use std::sync::atomic::Ordering;

pub struct ShieldModeAction;

#[async_trait]
impl Action for ShieldModeAction {
    type Settings = EmptySettings;
    const UUID: &'static str = "io.pngz.twitch.shieldmode";

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        if let Some(l) = &settings.button_label { crate::auth_handler::set_bold_title(instance, Some(l.as_str())).await?; }
        let Some((token, user_id, client_id)) = get_valid_token().await else { return Ok(()); };
        match twitch_api::get_shield_mode_status(&token, &client_id, &user_id, &user_id).await {
            Ok(s) => instance.set_state(if s.is_active { 1 } else { 0 }).await?,
            Err(e) => log::error!("get_shield_mode_status failed: {}", e),
        }
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        let Some((token, user_id, client_id)) = get_valid_token().await else {
            instance.show_alert().await?; return Ok(());
        };
        let active = instance.current_state_index.load(Ordering::Relaxed) == 1;
        match twitch_api::update_shield_mode(&token, &client_id, &user_id, &user_id, !active).await {
            Ok(_) => instance.set_state(if !active { 1 } else { 0 }).await?,
            Err(e) => { log::error!("update_shield_mode failed: {}", e); instance.show_alert().await?; }
        }
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
