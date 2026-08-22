#[tauri::command]
fn hello() -> &'static str {
    my_workspace_logger::debug!("hello command requested");
    my_workspace_cms_core::hello()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    my_workspace_logger::init().expect("failed to initialize logging");
    my_workspace_logger::info!("starting desktop application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![hello])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
