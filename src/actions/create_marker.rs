use crate::auth_handler::{get_auth, refresh_auth};
use crate::settings::EmptySettings;
use crate::twitch_api::{self, TwitchApiError};
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde_json::Value;

pub struct CreateMarkerAction;

#[async_trait]
impl Action for CreateMarkerAction {
    type Settings = EmptySettings;
    const UUID: &'static str = "io.pngz.twitch.createmarker";

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        crate::auth_handler::restore_title(instance, settings.button_label.as_deref()).await?;
        if let Some(img) = &settings.button_image {
            crate::auth_handler::set_button_image(instance, Some(img.as_str())).await?;
        }
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        let Some((token, user_id, client_id)) = get_auth(instance).await? else { return Ok(()); };
        let mut result = twitch_api::create_stream_marker(&token, &client_id, &user_id).await;
        if matches!(result, Err(TwitchApiError::Unauthorized)) {
            match refresh_auth(&instance.instance_id).await {
                Some((t2, uid2, cid2)) => result = twitch_api::create_stream_marker(&t2, &cid2, &uid2).await,
                None => { instance.show_alert().await?; return Ok(()); }
            }
        }
        match result {
            Ok(_) => instance.show_ok().await?,
            Err(e) => { log::error!("create_marker failed: {}", e); instance.show_alert().await?; }
        }
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
