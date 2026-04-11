use crate::auth::get_valid_token;
use crate::settings::EmptySettings;
use crate::twitch_api;
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde_json::Value;

pub struct ClearChatAction;

#[async_trait]
impl Action for ClearChatAction {
    type Settings = EmptySettings;
    const UUID: &'static str = "io.pngz.twitch.clearchat";

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        if let Some(l) = &settings.button_label { crate::auth_handler::set_bold_title(instance, Some(l.as_str())).await?; }
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        match get_valid_token().await {
            Some((token, user_id, client_id)) => {
                match twitch_api::clear_chat(&token, &client_id, &user_id, &user_id).await {
                    Ok(_) => instance.show_ok().await?,
                    Err(e) => { log::error!("clear_chat failed: {}", e); instance.show_alert().await?; }
                }
            }
            None => instance.show_alert().await?,
        }
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
