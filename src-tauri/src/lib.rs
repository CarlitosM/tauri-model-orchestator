use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Listener};

const ECHO_REQUEST_EVENT: &str = "frontend-echo-request";
const ECHO_RESPONSE_EVENT: &str = "backend-echo-response";
const OLLAMA_TAGS_URL: &str = "http://127.0.0.1:11434/api/tags";
const OLLAMA_CHAT_URL: &str = "http://127.0.0.1:11434/api/chat";

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

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaChatMessage,
}

#[derive(Clone, Serialize)]
struct NonStreamChatResponse {
    model: String,
    content: String,
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<OllamaModel>, String> {
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

#[tauri::command]
async fn non_stream_chat(model: String, message: String) -> Result<NonStreamChatResponse, String> {
    let model = model.trim().to_owned();
    let message = message.trim().to_owned();

    if model.is_empty() {
        return Err("Select an Ollama model before sending a chat message.".to_owned());
    }

    if message.is_empty() {
        return Err("Enter a chat message before sending it.".to_owned());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|error| format!("Could not configure the Ollama client: {error}"))?;
    let request = OllamaChatRequest {
        model,
        messages: vec![OllamaChatMessage {
            role: "user".to_owned(),
            content: message,
        }],
        stream: false,
    };

    let response = client
        .post(OLLAMA_CHAT_URL)
        .json(&request)
        .send()
        .await
        .map_err(|error| {
            format!(
                "Could not connect to Ollama at 127.0.0.1:11434. Make sure Ollama is running: {error}"
            )
        })?
        .error_for_status()
        .map_err(|error| format!("Ollama returned an unsuccessful response: {error}"))?;

    let response = response
        .json::<OllamaChatResponse>()
        .await
        .map_err(|error| format!("Could not read Ollama's chat response: {error}"))?;

    Ok(NonStreamChatResponse {
        model: response.model,
        content: response.message.content,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_ollama_models,
            non_stream_chat
        ])
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
    use super::{OllamaChatRequest, OllamaChatResponse, OllamaTagsResponse};

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

    #[test]
    fn serializes_a_non_streaming_chat_request() {
        let request = OllamaChatRequest {
            model: "llama3.2:latest".to_owned(),
            messages: vec![super::OllamaChatMessage {
                role: "user".to_owned(),
                content: "Hello".to_owned(),
            }],
            stream: false,
        };

        let value = serde_json::to_value(request).expect("chat request should serialize");

        assert_eq!(value["model"], "llama3.2:latest");
        assert_eq!(value["messages"][0]["role"], "user");
        assert_eq!(value["messages"][0]["content"], "Hello");
        assert_eq!(value["stream"], false);
    }

    #[test]
    fn deserializes_a_non_streaming_chat_response() {
        let response = serde_json::from_str::<OllamaChatResponse>(
            r#"{
                "model": "llama3.2:latest",
                "message": { "role": "assistant", "content": "Hi there!" },
                "done": true
            }"#,
        )
        .expect("chat response should deserialize");

        assert_eq!(response.model, "llama3.2:latest");
        assert_eq!(response.message.content, "Hi there!");
    }
}
