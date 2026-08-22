use crate::error::AppError;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

const OLLAMA_API_BASE_URL: &str = "http://localhost:11434/api/";
const OLLAMA_TAGS_ENDPOINT: &str = "tags";
const OLLAMA_CHAT_ENDPOINT: &str = "chat";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OllamaTag {
    name: String,
    size: u64,
    modified_at: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTag>,
}

pub async fn get_ollama_tags() -> Result<Vec<OllamaTag>, AppError> {
    let url = format!("{}{}", OLLAMA_API_BASE_URL, OLLAMA_TAGS_ENDPOINT);
    let response: OllamaTagsResponse = reqwest::get(&url).await?.json().await?;
    Ok(response.models)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OllamaChatResponse {
    pub model: String,
    pub message: OllamaChatMessage,
    pub done: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OllamaChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<OllamaChatMessage>,
}

pub async fn get_ollama_chat(
    chat: OllamaChatRequest,
) -> Result<OllamaChatResponse, AppError> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", OLLAMA_API_BASE_URL, OLLAMA_CHAT_ENDPOINT);
    let body =
        serde_json::json!({ "model": chat.model, "messages": chat.messages, "stream": false });
    let response = client.post(&url).json(&body).send().await?;
    let chat_response: OllamaChatResponse = response.json().await?;
    Ok(chat_response)
}

pub async fn stream_ollama_chat<F>(
    chat: OllamaChatRequest,
    mut handler: F,
) -> Result<(), AppError>
where
    F: FnMut(OllamaChatResponse) -> Result<(), AppError>,
{
    let client = reqwest::Client::new();
    let url = format!("{}{}", OLLAMA_API_BASE_URL, OLLAMA_CHAT_ENDPOINT);
    let body =
        serde_json::json!({ "model": chat.model, "messages": chat.messages, "stream": true });
    let response = client.post(&url).json(&body).send().await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chat_response: OllamaChatResponse = serde_json::from_slice(&chunk)?;
        handler(chat_response)?;
    }
    Ok(())
}
