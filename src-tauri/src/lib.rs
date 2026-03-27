mod app_config;
mod commands;
mod domain;
mod error;
mod runtime;
mod services;

use std::io;

use runtime::AppRuntime;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let runtime = AppRuntime::new(app.handle()).map_err(|error| {
                Box::new(io::Error::new(io::ErrorKind::Other, error.to_string()))
                    as Box<dyn std::error::Error>
            })?;
            runtime
                .logger
                .info("startup", "Ascension Addon Installer started.");
            app.manage(runtime);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrapApp,
            commands::inspectGamePath,
            commands::confirmGamePath,
            commands::refreshCatalog,
            commands::installAddon,
            commands::updateAddon,
            commands::updateAllAddons,
            commands::rollbackAddon,
            commands::checkInstallerUpdate,
            commands::openLogsFolder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
