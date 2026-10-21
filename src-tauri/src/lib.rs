pub mod audio;
pub mod chunker;
mod commands;
pub mod engine;
pub mod export;
pub mod model_manager;
mod self_test;
pub mod transcript;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, report] = args.as_slice()
        && flag == "--self-test"
    {
        std::process::exit(self_test::run(std::path::Path::new(report)));
    }

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([Target::new(TargetKind::Stdout), Target::new(TargetKind::LogDir { file_name: None })])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let models_dir = app.path().app_local_data_dir()?.join("models");
            log::info!("Wortlaut {} – models in {}", env!("CARGO_PKG_VERSION"), models_dir.display());
            app.manage(commands::AppState::new(models_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::model_info,
            commands::download_model,
            commands::cancel_download,
            commands::transcribe,
            commands::cancel_transcription,
            commands::export_transcript,
            commands::open_log_dir,
            commands::reveal_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
