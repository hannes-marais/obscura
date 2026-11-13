use serde_json::{json, Value};

use crate::dispatch::{CdpContext, DownloadBehavior};

pub async fn handle(method: &str, params: &Value, ctx: &mut CdpContext) -> Result<Value, String> {
    match method {
        "getVersion" => Ok(json!({
            "protocolVersion": "1.3",
            "product": "Chrome/145.0.0.0",
            "revision": "@0000000000000000000000000000000000000000",
            "userAgent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36",
            "jsVersion": "14.5.0.0",
        })),
        "close" => {
            Ok(json!({}))
        }
        "getWindowForTarget" => Ok(json!({
            "windowId": 1,
            "bounds": {
                "left": 0,
                "top": 0,
                "width": 1280,
                "height": 720,
                "windowState": "normal",
            }
        })),
        // Per browser context (the default context without browserContextId),
        // applied when a page starts a download (dispatch::drain_downloads).
        "setDownloadBehavior" => {
            let behavior = params["behavior"]
                .as_str()
                .ok_or("Invalid parameters: behavior: string value expected")?;
            if !matches!(behavior, "deny" | "allow" | "allowAndName" | "default") {
                return Err(format!("Unrecognized download behavior: {behavior}"));
            }
            let path = params["downloadPath"]
                .as_str()
                .filter(|path| !path.is_empty())
                .map(std::path::PathBuf::from);
            if matches!(behavior, "allow" | "allowAndName") {
                match &path {
                    Some(path) if path.is_absolute() => {}
                    Some(_) => return Err("downloadPath must be an absolute path".into()),
                    None => return Err("downloadPath must be provided if behavior is set to 'allow' or 'allowAndName'".into()),
                }
            }
            let context = match params["browserContextId"].as_str() {
                Some(id) if id == ctx.default_context.id || ctx.browser_contexts.contains_key(id) => id.to_string(),
                Some(id) => return Err(format!("Failed to find browser context for id {id}")),
                None => ctx.default_context.id.clone(),
            };
            ctx.download_behaviors.insert(
                context,
                DownloadBehavior {
                    behavior: behavior.to_string(),
                    path,
                    events: params["eventsEnabled"].as_bool().unwrap_or(false),
                },
            );
            Ok(json!({}))
        }
        // Downloads are written whole when the page starts them, so none is
        // ever in progress for a cancel to stop.
        "cancelDownload" => Ok(json!({})),
        "getWindowBounds" => Ok(json!({
            "bounds": { "left": 0, "top": 0, "width": 1280, "height": 720, "windowState": "normal" }
        })),
        // No-op acks for window-management methods Playwright sends during
        // page setup. We don't model real OS windows, but answering with {}
        // lets the client's setup sequence complete instead of tearing down
        // the page on an unknown-method error.
        "setWindowBounds" => Ok(json!({})),
        // Playwright grants permissions (geolocation, notifications, ...) per
        // browser context during setup. obscura does not gate any API on a
        // permission grant today, so the honest answer is to accept and
        // remember nothing; an unknown-method error would abort the client's
        // whole context initialization.
        "grantPermissions" | "resetPermissions" => Ok(json!({})),
        _ => Err(format!("Unknown Browser method: {}", method)),
    }
}
