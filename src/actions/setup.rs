use crate::auth::get_valid_token;
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct SetupAction;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SetupSettings {}

#[async_trait]
impl Action for SetupAction {
    type Settings = SetupSettings;
    const UUID: &'static str = "io.pngz.twitch.setup";

    async fn will_appear(&self, _instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        match get_valid_token().await {
            Some(_) => instance.show_ok().await?,
            None => instance.show_alert().await?,
        }
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
