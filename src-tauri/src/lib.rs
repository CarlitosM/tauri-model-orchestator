use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Listener};

const ECHO_REQUEST_EVENT: &str = "frontend-echo-request";
const ECHO_RESPONSE_EVENT: &str = "backend-echo-response";
const OLLAMA_TAGS_URL: &str = "http://127.0.0.1:11434/api/tags";

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

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTag>,
}

#[derive(Debug, Deserialize)]
struct OllamaTag {
    name: String,
    size: u64,
    modified_at: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OllamaModel {
    name: String,
    size: u64,
    modified_at: String,
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<OllamaModel>, String> {
    println!("Requesting Ollama models...");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| format!("Could not configure the Ollama client: {error}"))?;

    let response = client.get(OLLAMA_TAGS_URL).send().await.map_err(|error| {
        format!(
            "Could not connect to Ollama at 127.0.0.1:11434. Make sure Ollama is running: {error}"
        )
    })?;

    let response = response
        .error_for_status()
        .map_err(|error| format!("Ollama returned an unsuccessful response: {error}"))?;

    let tags = response
        .json::<OllamaTagsResponse>()
        .await
        .map_err(|error| format!("Could not read Ollama's model list: {error}"))?;

    Ok(tags
        .models
        .into_iter()
        .map(|model| OllamaModel {
            name: model.name,
            size: model.size,
            modified_at: model.modified_at,
        })
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_ollama_models])
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

#[cfg(test)]
mod tests {
    use super::OllamaTagsResponse;

    #[test]
    fn deserializes_ollama_tags_response() {
        let response = serde_json::from_str::<OllamaTagsResponse>(
            r#"{
                "models": [{
                    "name": "llama3.2:latest",
                    "model": "llama3.2:latest",
                    "modified_at": "2026-08-15T12:34:56.000000000Z",
                    "size": 2019393189,
                    "digest": "abc123",
                    "details": { "family": "llama" }
                }]
            }"#,
        )
        .expect("Ollama tags response should deserialize");

        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].name, "llama3.2:latest");
        assert_eq!(response.models[0].size, 2_019_393_189);
        assert_eq!(
            response.models[0].modified_at,
            "2026-08-15T12:34:56.000000000Z"
        );
    }
}
