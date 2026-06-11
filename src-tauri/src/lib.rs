#[tauri::command]
fn app_status() -> serde_json::Value {
    serde_json::json!({
        "capture": "paused",
        "encryption": "not_configured",
        "storage": "not_initialized"
    })
}

#[tauri::command]
fn list_sessions() -> Vec<String> {
    vec![
        "Market Research".to_string(),
        "Client Reply".to_string(),
        "Webhook Bug".to_string(),
    ]
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![app_status, list_sessions])
        .run(tauri::generate_context!())
        .expect("error while running ClipMind");
}
