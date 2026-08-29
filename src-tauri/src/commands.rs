use crate::error::AppError;
use crate::ollama::{
    get_ollama_chat, get_ollama_tags, OllamaChatMessage, OllamaChatRequest, OllamaTag,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ListModelsResponse(Vec<OllamaTag>);

#[tauri::command]
pub async fn list_models() -> Result<ListModelsResponse, AppError> {
    let tags = get_ollama_tags().await?;
    Ok(ListModelsResponse(tags))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NonStreamChatResponse {
    model: String,
    content: String,
}

#[tauri::command]
pub async fn non_stream_chat(
    model: String,
    message: String,
) -> Result<NonStreamChatResponse, AppError> {
    let chat_request = OllamaChatRequest {
        model,
        messages: vec![OllamaChatMessage {
            role: "user".to_owned(),
            content: message,
        }],
    };

    let response = get_ollama_chat(chat_request).await?;

    Ok(NonStreamChatResponse {
        model: response.model,
        content: response.message.content,
    })
}
