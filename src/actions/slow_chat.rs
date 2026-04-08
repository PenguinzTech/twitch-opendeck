use crate::auth::get_valid_token;
use crate::settings::SlowChatSettings;
use crate::twitch_api;
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

pub struct SlowChatAction;

#[async_trait]
impl Action for SlowChatAction {
    type Settings = SlowChatSettings;
    const UUID: &'static str = "dev.penguin.twitch.slowchat";

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        if let Some(l) = &settings.button_label { crate::auth_handler::set_bold_title(instance, Some(l.as_str())).await?; }
        let Some((token, user_id, client_id)) = get_valid_token().await else { return Ok(()); };
        match twitch_api::get_chat_settings(&token, &client_id, &user_id, &user_id).await {
            Ok(s) => instance.set_state(if s.slow_mode { 1 } else { 0 }).await?,
            Err(e) => log::error!("get_chat_settings failed: {}", e),
        }
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some((token, user_id, client_id)) = get_valid_token().await else {
            instance.show_alert().await?; return Ok(());
        };
        let active = instance.current_state_index.load(Ordering::Relaxed) == 1;
        match twitch_api::patch_chat_settings(&token, &client_id, &user_id, &user_id,
            json!({"slow_mode": !active, "slow_mode_wait_time": settings.wait_seconds})).await
        {
            Ok(_) => instance.set_state(if !active { 1 } else { 0 }).await?,
            Err(e) => { log::error!("patch slow_mode failed: {}", e); instance.show_alert().await?; }
        }
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
