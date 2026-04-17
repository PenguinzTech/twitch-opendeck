use crate::auth::{clear_auth_tokens, poll_for_token, refresh_access_token, start_device_flow, store_token, validate_token};
use crate::settings::{read_settings, save_settings, BUTTON_LABEL_MAX};
use openaction::{Instance, OpenActionResult, send_arbitrary_json};
use serde_json::json;

/// Set the button title with bold white styling, with multiline support.
pub async fn set_bold_title(instance: &Instance, title: Option<&str>) -> OpenActionResult<()> {
    match title.filter(|t| !t.is_empty()) {
        None => instance.set_title(None::<&str>, None).await,
        Some(t) => {
            crate::settings::cache_title(&instance.instance_id, t).await;
            let multiline = t.contains('\n');
            send_arbitrary_json(json!({
                "event": "setTitle",
                "context": instance.instance_id,
                "payload": {
                    "title": t,
                    "titleParameters": {
                        "fontFamily": "",
                        "fontSize": if multiline { 10 } else { 14 },
                        "fontStyle": "Bold",
                        "fontUnderline": false,
                        "showTitle": true,
                        "titleAlignment": if multiline { "middle" } else { "bottom" },
                        "titleColor": "#ffffff"
                    }
                }
            }))
            .await
        }
    }
}

/// Restore the button title from settings or cache on will_appear.
pub async fn restore_title(instance: &Instance, label_from_settings: Option<&str>) -> OpenActionResult<()> {
    if let Some(l) = label_from_settings.filter(|t| !t.is_empty()) {
        return set_bold_title(instance, Some(l)).await;
    }
    if let Some(cached) = crate::settings::get_cached_title(&instance.instance_id).await {
        if !cached.is_empty() {
            return set_bold_title(instance, Some(&cached)).await;
        }
    }
    Ok(())
}

/// Set the button image from a base64 data URI string.
pub async fn set_button_image(instance: &Instance, image_data: Option<&str>) -> OpenActionResult<()> {
    send_arbitrary_json(json!({
        "event": "setImage",
        "context": instance.instance_id,
        "payload": {
            "image": image_data.unwrap_or(""),
            "target": 0
        }
    }))
    .await
}

/// Send a reauth_required event to the PI for a specific instance.
/// This prompts the PI to show the re-authentication UI.
pub async fn notify_reauth_needed(instance_id: &str) {
    if let Some(inst) = openaction::get_instance(instance_id.to_string()).await {
        let _ = inst
            .send_to_property_inspector(json!({"event": "reauth_required"}))
            .await;
    }
}

/// Get a valid auth token for an action. If auth is unavailable, notifies the PI
/// and shows an alert on the button. Returns None if the action should abort.
pub async fn get_auth(instance: &Instance) -> OpenActionResult<Option<(String, String, String)>> {
    match crate::auth::get_valid_token().await {
        Some(tok) => Ok(Some(tok)),
        None => {
            notify_reauth_needed(&instance.instance_id).await;
            instance.show_alert().await?;
            Ok(None)
        }
    }
}

/// After receiving a 401 from the API, attempt a token refresh.
/// Returns new credentials if refresh succeeded (caller should retry the API call).
/// Returns None if refresh failed — auth is cleared and PI is notified to re-auth.
pub async fn refresh_auth(instance_id: &str) -> Option<(String, String, String)> {
    let settings = read_settings().await;
    let rt = match settings.refresh_token.clone() {
        Some(rt) => rt,
        None => {
            log::warn!("No refresh token available after 401");
            clear_auth_tokens().await;
            notify_reauth_needed(instance_id).await;
            return None;
        }
    };

    match refresh_access_token(&settings.client_id, &settings.client_secret, &rt).await {
        Ok(new_token) => {
            let access = new_token.access_token.clone();
            let user_id = settings.user_id.clone().unwrap_or_default();
            let client_id = settings.client_id.clone();
            let expires_at = chrono::Utc::now().timestamp() + new_token.expires_in.unwrap_or(14400);
            let mut updated = settings;
            updated.access_token = Some(access.clone());
            updated.refresh_token = new_token.refresh_token;
            updated.token_expires_at = Some(expires_at);
            let _ = save_settings(updated).await;
            log::info!("Token refreshed successfully after 401");
            Some((access, user_id, client_id))
        }
        Err(e) => {
            log::warn!("Token refresh failed after 401: {}", e);
            clear_auth_tokens().await;
            notify_reauth_needed(instance_id).await;
            None
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
            let instance_id = instance.instance_id.clone();
            tokio::spawn(async move {
                let (authenticated, username, client_id) = if s.is_authenticated() {
                    if let Some(token) = s.access_token.as_deref() {
                        match validate_token(token).await {
                            Ok(v) => (true, Some(v.login), s.client_id.clone()),
                            Err(_) => {
                                // Token validation failed — try to refresh before giving up
                                if let Some(rt) = s.refresh_token.as_deref() {
                                    match refresh_access_token(&s.client_id, &s.client_secret, rt).await {
                                        Ok(new_tok) => {
                                            match validate_token(&new_tok.access_token).await {
                                                Ok(v) => {
                                                    let expires_at = chrono::Utc::now().timestamp()
                                                        + new_tok.expires_in.unwrap_or(14400);
                                                    let mut updated = s.clone();
                                                    updated.access_token = Some(new_tok.access_token);
                                                    updated.refresh_token = new_tok.refresh_token;
                                                    updated.token_expires_at = Some(expires_at);
                                                    updated.user_id = Some(v.user_id);
                                                    updated.username = Some(v.login.clone());
                                                    let _ = save_settings(updated).await;
                                                    (true, Some(v.login), s.client_id.clone())
                                                }
                                                Err(_) => {
                                                    clear_auth_tokens().await;
                                                    (false, None, s.client_id.clone())
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            clear_auth_tokens().await;
                                            (false, None, s.client_id.clone())
                                        }
                                    }
                                } else {
                                    clear_auth_tokens().await;
                                    (false, None, s.client_id.clone())
                                }
                            }
                        }
                    } else {
                        (false, None, s.client_id.clone())
                    }
                } else {
                    (false, None, s.client_id.clone())
                };

                if let Some(inst) = openaction::get_instance(instance_id).await {
                    let _ = inst.send_to_property_inspector(json!({
                        "event": "auth_status",
                        "authenticated": authenticated,
                        "username": username,
                        "client_id": client_id
                    })).await;
                }
            });
        }
        "force_reauth" | "logout" => {
            // Clear tokens but preserve client credentials so user doesn't have to re-enter them
            clear_auth_tokens().await;
            let client_id = read_settings().await.client_id;
            instance.send_to_property_inspector(json!({
                "event": "auth_status",
                "authenticated": false,
                "username": null,
                "client_id": client_id
            })).await?;
        }
        "set_title" => {
            let raw = payload.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let title: String = raw.lines().take(2)
                .map(|line| line.chars().take(BUTTON_LABEL_MAX).collect::<String>())
                .collect::<Vec<_>>().join("\n");
            set_bold_title(instance, if title.is_empty() { None } else { Some(&title) }).await?;
            instance.send_to_property_inspector(json!({
                "event": "title_set",
                "title": title
            })).await?;
        }
        "set_image" => {
            let data = payload.get("image_data").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let instance_id = instance.instance_id.clone();
            tokio::spawn(async move {
                if let Some(inst) = openaction::get_instance(instance_id).await {
                    let _ = set_button_image(&inst, if data.is_empty() { None } else { Some(&data) }).await;
                }
            });
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
