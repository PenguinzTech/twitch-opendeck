use crate::auth_handler::{get_auth, refresh_auth};
use crate::settings::EmptySettings;
use crate::twitch_api::{self, TwitchApiError};
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

pub struct EmoteChatAction;

#[async_trait]
impl Action for EmoteChatAction {
    type Settings = EmptySettings;
    const UUID: &'static str = "io.pngz.twitch.emotechat";

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        crate::auth_handler::restore_title(instance, settings.button_label.as_deref()).await?;
        if let Some(img) = &settings.button_image {
            crate::auth_handler::set_button_image(instance, Some(img.as_str())).await?;
        }
        if let Some((token, user_id, client_id)) = crate::auth::get_valid_token().await {
            match twitch_api::get_chat_settings(&token, &client_id, &user_id, &user_id).await {
                Ok(s) => instance.set_state(if s.emote_mode { 1 } else { 0 }).await?,
                Err(e) => log::error!("get_chat_settings failed: {}", e),
            }
        }
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        let active = instance.current_state_index.load(Ordering::Relaxed) == 1;
        let Some((token, user_id, client_id)) = get_auth(instance).await? else { return Ok(()); };
        let body = json!({"emote_mode": !active});
        let mut result = twitch_api::patch_chat_settings(&token, &client_id, &user_id, &user_id, body.clone()).await;
        if matches!(result, Err(TwitchApiError::Unauthorized)) {
            match refresh_auth(&instance.instance_id).await {
                Some((t2, uid2, cid2)) => result = twitch_api::patch_chat_settings(&t2, &cid2, &uid2, &uid2, body).await,
                None => { instance.show_alert().await?; return Ok(()); }
            }
        }
        match result {
            Ok(_) => instance.set_state(if !active { 1 } else { 0 }).await?,
            Err(e) => { log::error!("patch emote_mode failed: {}", e); instance.show_alert().await?; }
        }
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
