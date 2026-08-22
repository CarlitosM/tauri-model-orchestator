use crate::error::AppError;
use crate::ollama::{stream_ollama_chat, OllamaChatRequest, OllamaChatResponse};
use serde::Serialize;
use tauri::{ipc::Channel, AppHandle};

#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum StreamChatEvent {
    ChatResponse(OllamaChatResponse),
    ChatFinished(OllamaChatResponse),
}

#[tauri::command]
pub async fn stream_chat(
    _: AppHandle,
    request: OllamaChatRequest,
    on_event: Channel<StreamChatEvent>,
) -> Result<(), AppError> {
    stream_ollama_chat(request, |response| {
        let event = if response.done {
            StreamChatEvent::ChatFinished(response)
        } else {
            StreamChatEvent::ChatResponse(response)
        };
        on_event.send(event)?;
        Ok(())
    })
    .await
}
