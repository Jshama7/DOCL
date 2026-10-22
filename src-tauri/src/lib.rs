mod auth;
mod commands;
mod config;
mod download;
mod http;
mod install;
mod java;
mod launch;
mod mirror;
mod models;

use std::sync::Mutex;

pub struct AppState {
    pub config: Mutex<config::Config>,
    pub client: reqwest::Client,
    pub progress: Mutex<Option<std::sync::Arc<download::ProgressState>>>,
    pub manifest: Mutex<Option<models::VersionManifest>>,
    pub child_pid: Mutex<Option<u32>>,
    pub accounts: Mutex<Vec<auth::Account>>,
}

pub fn run() {
    let cfg = config::load();
    let accounts = auth::load_accounts();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(cfg),
            client: http::client(),
            progress: Mutex::new(None),
            manifest: Mutex::new(None),
            child_pid: Mutex::new(None),
            accounts: Mutex::new(accounts),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::fetch_versions,
            commands::installed_versions,
            commands::install_version,
            commands::cancel_install,
            commands::loader_options,
            commands::install_loader,
            commands::delete_version,
            commands::get_accounts,
            commands::create_offline_account,
            commands::remove_account,
            commands::select_account,
            commands::ms_login_begin,
            commands::detect_java,
            commands::install_java,
            commands::launch_version,
            commands::kill_game,
            commands::import_background,
            commands::background_data_url,
        ])
        .run(tauri::generate_context!())
        .expect("启动器运行失败");
}
