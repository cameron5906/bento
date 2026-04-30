mod commands;
mod supervisor_client;

use commands::SupervisorState;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(SupervisorState {
            client: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::send_command,
            commands::connect_supervisor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
