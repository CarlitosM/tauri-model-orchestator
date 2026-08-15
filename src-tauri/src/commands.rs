#[tauri::command]
pub fn say_hello(to: String) -> String {
    format!("Hello {to}!")
}
