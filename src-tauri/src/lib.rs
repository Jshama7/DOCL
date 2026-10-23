mod auth;
mod appupdate;
mod commands;
mod config;
mod crash;
mod curseforge;
mod download;
mod dropzone;
mod forge_native;
mod hall;
mod http;
mod install;
mod instance;
mod java;
mod lan;
mod launch;
mod mirror;
mod modpack;
mod moddeps;
mod modrinth;
mod models;
mod news;
mod ping;
mod server;
mod skins;
mod system;
mod terracotta;
mod update;

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
    lan::start();

    let app = tauri::Builder::default()
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
            commands::system_memory,
            commands::news_feed,
            commands::news_article,
            commands::save_config,
            commands::export_settings,
            commands::import_settings,
            commands::app_version,
            commands::update_check,
            commands::update_download,
            commands::update_install,
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
            commands::instance_set_icon,
            commands::instance_set_icon_image,
            commands::instance_icon_image,
            commands::instance_files,
            commands::instance_toggle_file,
            commands::instance_delete_file,
            commands::instance_thumb,
            commands::instance_open_content,
            commands::instance_import_files,
            commands::drop_inspect,
            commands::instance_import_mods,
            commands::instance_paths,
            commands::instance_worlds,
            commands::instance_world_seed,
            commands::modpack_inspect,
            commands::modpack_download,
            commands::modpack_temp_remove,
            commands::modpack_import,
            commands::modpack_export_preview,
            commands::modpack_export,
            commands::instance_context,
            commands::crash_analyze,
            commands::mod_updates,
            commands::mod_update_install,
            commands::cf_search,
            commands::cf_game_versions,
            commands::cf_files,
            commands::modrinth_search,
            commands::modrinth_files,
            commands::content_projects,
            commands::cf_install_file,
            commands::default_skin,
            commands::default_skins,
            commands::get_accounts,
            commands::create_offline_account,
            commands::remove_account,
            commands::select_account,
            commands::get_account_skin,
            commands::save_account_skin,
            commands::clear_account_skin,
            commands::export_account_skin,
            commands::upload_account_skin,
            commands::skin_library_list,
            commands::skin_library_import,
            commands::skin_library_save,
            commands::skin_library_from_account,
            commands::skin_library_from_player,
            commands::skin_library_rename,
            commands::skin_library_remove,
            commands::skin_library_apply,
            commands::account_capes,
            commands::cape_set_active,
            commands::ms_login_begin,
            commands::detect_java,
            commands::install_java,
            commands::launch_version,
            commands::launch_server,
            commands::lan_rooms,
            commands::hall_rooms,
            commands::server_versions,
            commands::server_status,
            commands::server_start,
            commands::server_command,
            commands::server_stop,
            commands::terracotta_prepare,
            commands::terracotta_status,
            commands::terracotta_host,
            commands::terracotta_join,
            commands::terracotta_reset,
            commands::terracotta_shutdown,
            commands::remember_server,
            commands::forget_server,
            commands::server_ping,
            commands::kill_game,
            commands::import_background,
            commands::background_data_url,
        ])
        .build(tauri::generate_context!())
        .expect("构建启动器失败");

    app.run(|_app, event| {
        if let tauri::RunEvent::Exit = event {
            terracotta::shutdown_blocking();
        }
    });
}
