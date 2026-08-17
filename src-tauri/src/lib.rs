use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Listener};

const STREAMING_CHAT_EVENT: &str = "streaming-chat";
const STREAMING_CHAT_RESPONSE_EVENT: &str = "streaming-chat-response";
const OLLAMA_TAGS_URL: &str = "http://127.0.0.1:11434/api/tags";
const OLLAMA_CHAT_URL: &str = "http://127.0.0.1:11434/api/chat";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamingChatRequest {
    request_id: String,
    model: String,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamingChatResponse {
    request_id: String,
    model: String,
    content: String,
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
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
    done: bool,
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

fn emit_stream_response(app_handle: &tauri::AppHandle, response: StreamingChatResponse) -> bool {
    if let Err(error) = app_handle.emit_to("main", STREAMING_CHAT_RESPONSE_EVENT, response) {
        log::error!("Could not emit streaming chat response: {error}");
        return false;
    }

    true
}

fn emit_stream_error(
    app_handle: &tauri::AppHandle,
    request_id: &str,
    model: &str,
    error: impl Into<String>,
) {
    let _ = emit_stream_response(
        app_handle,
        StreamingChatResponse {
            request_id: request_id.to_owned(),
            model: model.to_owned(),
            content: String::new(),
            done: true,
            error: Some(error.into()),
        },
    );
}

fn take_ndjson_records(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut records = Vec::new();

    while let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
        records.push(buffer.drain(..=end).collect());
    }

    records
}

fn forward_stream_record(
    app_handle: &tauri::AppHandle,
    request_id: &str,
    record: &[u8],
) -> Result<bool, String> {
    if record.iter().all(u8::is_ascii_whitespace) {
        return Ok(false);
    }

    let response = serde_json::from_slice::<OllamaChatResponse>(record)
        .map_err(|error| format!("Could not read Ollama's streaming chat response: {error}"))?;
    let done = response.done;
    let response = StreamingChatResponse {
        request_id: request_id.to_owned(),
        model: response.model,
        content: response.message.content,
        done,
        error: None,
    };

    if !emit_stream_response(app_handle, response) {
        return Err("Could not deliver Ollama's streaming response to the frontend.".to_owned());
    }

    Ok(done)
}

async fn stream_chat(app_handle: tauri::AppHandle, request: StreamingChatRequest) {
    let request_id = request.request_id.trim().to_owned();
    let model = request.model.trim().to_owned();
    let message = request.message.trim().to_owned();

    if request_id.is_empty() {
        emit_stream_error(
            &app_handle,
            &request_id,
            &model,
            "A request ID is required for streaming chat.",
        );
        return;
    }

    if model.is_empty() {
        emit_stream_error(
            &app_handle,
            &request_id,
            &model,
            "Select an Ollama model before sending a chat message.",
        );
        return;
    }

    if message.is_empty() {
        emit_stream_error(
            &app_handle,
            &request_id,
            &model,
            "Enter a chat message before sending it.",
        );
        return;
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            emit_stream_error(
                &app_handle,
                &request_id,
                &model,
                format!("Could not configure the Ollama client: {error}"),
            );
            return;
        }
    };
    let request = OllamaChatRequest {
        model: model.clone(),
        messages: vec![OllamaChatMessage {
            role: "user".to_owned(),
            content: message,
        }],
        stream: true,
    };

    let mut response = match client.post(OLLAMA_CHAT_URL).json(&request).send().await {
        Ok(response) => response,
        Err(error) => {
            emit_stream_error(
                &app_handle,
                &request_id,
                &model,
                format!(
                    "Could not connect to Ollama at 127.0.0.1:11434. Make sure Ollama is running: {error}"
                ),
            );
            return;
        }
    };

    response = match response.error_for_status() {
        Ok(response) => response,
        Err(error) => {
            emit_stream_error(
                &app_handle,
                &request_id,
                &model,
                format!("Ollama returned an unsuccessful response: {error}"),
            );
            return;
        }
    };

    let mut buffer = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                buffer.extend_from_slice(&chunk);

                for record in take_ndjson_records(&mut buffer) {
                    match forward_stream_record(&app_handle, &request_id, &record) {
                        Ok(true) => return,
                        Ok(false) => {}
                        Err(error) => {
                            emit_stream_error(&app_handle, &request_id, &model, error);
                            return;
                        }
                    }
                }
            }
            Ok(None) => break,
            Err(error) => {
                emit_stream_error(
                    &app_handle,
                    &request_id,
                    &model,
                    format!("Could not read Ollama's streaming response: {error}"),
                );
                return;
            }
        }
    }

    if !buffer.iter().all(u8::is_ascii_whitespace) {
        match forward_stream_record(&app_handle, &request_id, &buffer) {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                emit_stream_error(&app_handle, &request_id, &model, error);
                return;
            }
        }
    }

    emit_stream_error(
        &app_handle,
        &request_id,
        &model,
        "Ollama ended the streaming response without a completion record.",
    );
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
            app.listen(STREAMING_CHAT_EVENT, move |event| {
                let request = match serde_json::from_str::<StreamingChatRequest>(event.payload()) {
                    Ok(request) => request,
                    Err(error) => {
                        log::warn!("Ignoring malformed streaming chat request: {error}");
                        return;
                    }
                };

                tauri::async_runtime::spawn(stream_chat(app_handle.clone(), request));
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        take_ndjson_records, OllamaChatRequest, OllamaChatResponse, OllamaTagsResponse,
        StreamingChatResponse,
    };

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
    fn serializes_a_streaming_chat_request() {
        let request = OllamaChatRequest {
            model: "llama3.2:latest".to_owned(),
            messages: vec![super::OllamaChatMessage {
                role: "user".to_owned(),
                content: "Hello".to_owned(),
            }],
            stream: true,
        };

        let value = serde_json::to_value(request).expect("chat request should serialize");

        assert_eq!(value["stream"], true);
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
        assert!(response.done);
    }

    #[test]
    fn buffers_ndjson_records_across_transport_chunks() {
        let mut buffer = b"{\"model\":\"llama\",\"message\":{\"role\":\"assistant\",\"content\":\"Hi\"},\"done\":false}\n{\"model\":\"llama\"".to_vec();

        let records = take_ndjson_records(&mut buffer);

        assert_eq!(records.len(), 1);
        assert!(buffer.starts_with(b"{\"model\":\"llama\""));

        buffer.extend_from_slice(
            b",\"message\":{\"role\":\"assistant\",\"content\":\"!\"},\"done\":true}\n",
        );
        let records = take_ndjson_records(&mut buffer);

        assert_eq!(records.len(), 1);
        assert!(buffer.is_empty());
        let response = serde_json::from_slice::<OllamaChatResponse>(&records[0])
            .expect("streaming record should deserialize");
        assert_eq!(response.message.content, "!");
        assert!(response.done);
    }

    #[test]
    fn serializes_a_terminal_streaming_error_response() {
        let response = StreamingChatResponse {
            request_id: "request-1".to_owned(),
            model: "llama3.2:latest".to_owned(),
            content: String::new(),
            done: true,
            error: Some("Ollama is unavailable".to_owned()),
        };

        let value = serde_json::to_value(response).expect("streaming response should serialize");

        assert_eq!(value["requestId"], "request-1");
        assert_eq!(value["model"], "llama3.2:latest");
        assert_eq!(value["content"], "");
        assert_eq!(value["done"], true);
        assert_eq!(value["error"], "Ollama is unavailable");
    }
}
