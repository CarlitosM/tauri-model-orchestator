use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Listener};

const ECHO_REQUEST_EVENT: &str = "frontend-echo-request";
const ECHO_RESPONSE_EVENT: &str = "backend-echo-response";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EchoRequest {
    request_id: String,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EchoResponse {
    request_id: String,
    message: String,
    received_at: u64,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_handle = app.handle().clone();
            app.listen(ECHO_REQUEST_EVENT, move |event| {
                let request = match serde_json::from_str::<EchoRequest>(event.payload()) {
                    Ok(request) if !request.message.trim().is_empty() => request,
                    Ok(_) => {
                        log::warn!("Ignoring an empty echo request");
                        return;
                    }
                    Err(error) => {
                        log::warn!("Ignoring malformed echo request: {error}");
                        return;
                    }
                };

                let received_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
                    .unwrap_or_default();
                let response = EchoResponse {
                    request_id: request.request_id,
                    message: request.message,
                    received_at,
                };

                if let Err(error) = app_handle.emit_to("main", ECHO_RESPONSE_EVENT, response) {
                    log::error!("Could not emit echo response: {error}");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
