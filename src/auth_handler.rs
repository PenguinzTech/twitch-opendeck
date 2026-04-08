use crate::auth::{poll_for_token, start_device_flow, store_token, validate_token};
use crate::settings::{read_settings, save_settings, BUTTON_LABEL_MAX};
use openaction::{Instance, OpenActionResult, send_arbitrary_json};
use serde_json::json;

/// Set the button title with bold white styling.
pub async fn set_bold_title(instance: &Instance, title: Option<&str>) -> OpenActionResult<()> {
    match title.filter(|t| !t.is_empty()) {
        None => instance.set_title(None::<&str>, None).await,
        Some(t) => {
            send_arbitrary_json(json!({
                "event": "setTitle",
                "context": instance.instance_id,
                "payload": {
                    "title": t,
                    "titleParameters": {
                        "fontFamily": "",
                        "fontSize": 14,
                        "fontStyle": "Bold",
                        "fontUnderline": false,
                        "showTitle": true,
                        "titleAlignment": "bottom",
                        "titleColor": "#ffffff"
                    }
                }
            }))
            .await
        }
    }
}

/// Shared handler for auth-related PI messages. Call this from send_to_plugin in every action.
pub async fn handle_auth_message(
    instance: &Instance,
    payload: &serde_json::Value,
) -> OpenActionResult<()> {
    match payload.get("event").and_then(|e| e.as_str()).unwrap_or("") {
        "start_auth" => {
            let s = read_settings().await;
            let client_id = s.client_id.clone();
            let client_secret = s.client_secret.clone();
            let instance_id = instance.instance_id.clone();
            tokio::spawn(async move {
                if let Err(e) = run_auth_flow(&instance_id, &client_id, &client_secret).await {
                    log::error!("Auth flow error: {}", e);
                    if let Some(inst) = openaction::get_instance(instance_id.clone()).await {
                        let _ = inst
                            .send_to_property_inspector(
                                json!({"event": "auth_error", "message": e.to_string()}),
                            )
                            .await;
                    }
                }
            });
        }
        "save_credentials" => {
            let cid = payload.get("client_id").and_then(|c| c.as_str()).unwrap_or("").to_string();
            let csec = payload.get("client_secret").and_then(|c| c.as_str()).unwrap_or("").to_string();
            if !cid.is_empty() {
                let mut s = read_settings().await;
                s.client_id = cid;
                s.client_secret = csec;
                let _ = save_settings(s).await;
                instance
                    .send_to_property_inspector(json!({"event": "credentials_saved"}))
                    .await?;
            }
        }
        "get_auth_status" => {
            let s = read_settings().await;
            instance
                .send_to_property_inspector(json!({
                    "event": "auth_status",
                    "authenticated": s.is_authenticated(),
                    "username": s.username,
                    "client_id": s.client_id
                }))
                .await?;
        }
        "set_title" => {
            let raw = payload.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let title: String = raw.chars().take(BUTTON_LABEL_MAX).collect();
            set_bold_title(instance, if title.is_empty() { None } else { Some(&title) }).await?;
            instance.send_to_property_inspector(json!({
                "event": "title_set",
                "title": title
            })).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn run_auth_flow(
    instance_id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    if client_id.is_empty() {
        return Err("Client ID not set. Enter your Twitch Application credentials first.".into());
    }
    if client_secret.is_empty() {
        return Err("Client Secret not set. Enter your Twitch Application credentials first.".into());
    }
    let d = start_device_flow(client_id).await?;
    if let Some(inst) = openaction::get_instance(instance_id.to_string()).await {
        inst.send_to_property_inspector(json!({
            "event": "auth_started",
            "user_code": d.user_code,
            "verification_uri": d.verification_uri
        }))
        .await?;
    }
    let token = poll_for_token(client_id, client_secret, &d.device_code, d.interval, d.expires_in).await?;
    let v = validate_token(&token.access_token).await?;
    let username = v.login.clone();
    store_token(token, v).await?;
    if let Some(inst) = openaction::get_instance(instance_id.to_string()).await {
        inst.send_to_property_inspector(json!({
            "event": "auth_success",
            "username": username
        }))
        .await?;
    }
    Ok(())
}
