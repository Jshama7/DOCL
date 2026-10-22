mod auth;
mod commands;
mod config;
mod curseforge;
mod download;
mod http;
mod install;
mod instance;
mod java;
mod launch;
mod mirror;
mod modpack;
mod modrinth;
mod models;
mod skins;

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
            commands::open_folder,
            commands::reveal_file,
            commands::game_dirs,
            commands::add_game_dir,
            commands::switch_game_dir,
            commands::remove_game_dir,
            commands::fetch_versions,
            commands::installed_versions,
            commands::install_version,
            commands::cancel_install,
            commands::loader_options,
            commands::loader_versions,
            commands::install_loader,
            commands::delete_version,
            commands::instance_info,
            commands::instance_metas,
            commands::instance_save_meta,
            commands::instance_files,
            commands::instance_toggle_file,
            commands::instance_delete_file,
            commands::instance_import_files,
            commands::instance_paths,
            commands::modpack_inspect,
            commands::modpack_import,
            commands::modpack_export_preview,
            commands::modpack_export,
            commands::instance_context,
            commands::cf_search,
            commands::cf_game_versions,
            commands::cf_files,
            commands::modrinth_search,
            commands::modrinth_files,
            commands::cf_install_file,
            commands::get_accounts,
            commands::create_offline_account,
            commands::remove_account,
            commands::select_account,
            commands::get_account_skin,
            commands::save_account_skin,
            commands::clear_account_skin,
            commands::export_account_skin,
            commands::upload_account_skin,
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
