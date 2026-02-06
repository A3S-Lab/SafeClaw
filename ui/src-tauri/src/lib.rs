use tauri::Manager;

#[tauri::command]
fn get_gateway_url() -> String {
    std::env::var("SAFECLAW_GATEWAY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:18790".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![get_gateway_url])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running SafeClaw");
}
