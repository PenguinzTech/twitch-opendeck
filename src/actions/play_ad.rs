use crate::auth_handler::{get_auth, refresh_auth};
use crate::settings::PlayAdSettings;
use crate::twitch_api::{self, TwitchApiError};
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde_json::Value;

pub struct PlayAdAction;

#[async_trait]
impl Action for PlayAdAction {
    type Settings = PlayAdSettings;
    const UUID: &'static str = "io.pngz.twitch.playad";

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        crate::auth_handler::restore_title(instance, settings.button_label.as_deref()).await?;
        if let Some(img) = &settings.button_image {
            crate::auth_handler::set_button_image(instance, Some(img.as_str())).await?;
        }
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let duration = settings.duration_seconds;
        let Some((token, user_id, client_id)) = get_auth(instance).await? else { return Ok(()); };
        let mut result = twitch_api::start_commercial(&token, &client_id, &user_id, duration).await;
        if matches!(result, Err(TwitchApiError::Unauthorized)) {
            match refresh_auth(&instance.instance_id).await {
                Some((t2, uid2, cid2)) => result = twitch_api::start_commercial(&t2, &cid2, &uid2, duration).await,
                None => { instance.show_alert().await?; return Ok(()); }
            }
        }
        match result {
            Ok(_) => instance.show_ok().await?,
            Err(e) => { log::error!("start_commercial failed: {}", e); instance.show_alert().await?; }
        }
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
