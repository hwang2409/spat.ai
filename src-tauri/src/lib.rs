mod commands;
mod pipeline;

use commands::PipelineState;
use std::sync::Mutex;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spat_ai=debug,tft_capture=debug,tft_vision=debug".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(PipelineState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::start_capture,
            commands::stop_capture,
            commands::get_capture_status,
            commands::get_game_state,
            commands::list_windows,
            commands::set_target_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
