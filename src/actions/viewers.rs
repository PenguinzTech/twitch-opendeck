use crate::auth::get_valid_token;
use crate::settings::EmptySettings;
use crate::twitch_api::{self, TwitchApiError};
use openaction::{Action, Instance, OpenActionResult, async_trait};
use serde_json::Value;

pub struct ViewersAction;

fn format_viewers(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[async_trait]
impl Action for ViewersAction {
    type Settings = EmptySettings;
    const UUID: &'static str = "io.pngz.twitch.viewers";

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        if let Some(img) = &settings.button_image {
            crate::auth_handler::set_button_image(instance, Some(img.as_str())).await?;
        }
        match get_valid_token().await {
            Some((token, user_id, client_id)) => {
                match twitch_api::get_stream_info(&token, &client_id, &user_id).await {
                    Ok(Some(info)) => instance.set_title(Some(format_viewers(info.viewer_count)), None).await?,
                    Ok(None) => instance.set_title(Some("Offline"), None).await?,
                    Err(_) => instance.set_title(Some("N/A"), None).await?,
                }
            }
            None => instance.set_title(Some("N/A"), None).await?,
        }

        let instance_id = instance.instance_id.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            interval.tick().await;
            loop {
                interval.tick().await;
                let Some(inst) = openaction::get_instance(instance_id.clone()).await else { break };

                let Some((token, user_id, client_id)) = get_valid_token().await else {
                    // No valid token — stop polling silently
                    break;
                };

                let result = twitch_api::get_stream_info(&token, &client_id, &user_id).await;

                let result = if matches!(result, Err(TwitchApiError::Unauthorized)) {
                    // Token revoked mid-session — try refresh once
                    match crate::auth_handler::refresh_auth(&instance_id).await {
                        Some((t2, uid2, cid2)) => twitch_api::get_stream_info(&t2, &cid2, &uid2).await,
                        None => break, // Refresh failed; auth cleared and PI notified — stop polling
                    }
                } else {
                    result
                };

                match result {
                    Ok(Some(info)) => { let _ = inst.set_title(Some(format_viewers(info.viewer_count)), None).await; }
                    Ok(None) => { let _ = inst.set_title(Some("Offline"), None).await; }
                    Err(_) => {} // Transient error — keep polling
                }
            }
        });
        Ok(())
    }

    async fn send_to_plugin(&self, instance: &Instance, _settings: &Self::Settings, payload: &Value) -> OpenActionResult<()> {
        crate::auth_handler::handle_auth_message(instance, payload).await
    }
}
