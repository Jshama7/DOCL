use crate::auth::{self, Account, AccountView, DeviceCode};
use crate::config::{self, Config};
use crate::curseforge;
use crate::download::{self, ProgressState};
use crate::install::{self, InstalledVersion, LoaderOption};
use crate::instance::{self, InstanceFile, InstanceInfo, InstanceMeta, InstancePaths};
use crate::java::{self, JavaInfo};
use crate::launch;
use crate::models::*;
use crate::modpack;
use crate::skins;
use crate::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub fn open_folder(path: String) -> CmdResult<()> {
    let target = std::path::PathBuf::from(path.trim());
    if !target.exists() {
        return Err(format!("目录不存在：{}", target.display()));
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&target)
            .spawn()
            .map_err(err)?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(&target)
            .spawn()
            .map_err(err)?;
        Ok(())
    }
}

#[tauri::command]
pub fn reveal_file(path: String) -> CmdResult<()> {
    let target = std::path::PathBuf::from(path.trim());
    if !target.exists() {
        return Err(format!("文件不存在：{}", target.display()));
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&target)
            .spawn()
            .map_err(err)?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        if let Some(parent) = target.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(err)?;
        }
        Ok(())
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> CmdResult<Config> {
    let mut config = config;
    config.sync_game_dirs();
    config::save(&config).map_err(err)?;
    std::fs::create_dir_all(&config.game_dir).map_err(err)?;
    *state.config.lock().unwrap() = config.clone();
    Ok(config)
}

#[derive(serde::Serialize)]
pub struct GameDirsView {
    pub dirs: Vec<config::GameDirEntry>,
    pub active: String,
    pub official: String,
}

#[tauri::command]
pub fn game_dirs(state: State<'_, AppState>) -> GameDirsView {
    let cfg = state.config.lock().unwrap().clone();
    GameDirsView {
        dirs: cfg.game_dirs.clone(),
        active: cfg.game_dir.clone(),
        official: config::default_game_dir().to_string_lossy().to_string(),
    }
}

/// 添加或切换目录；path 会先创建再写入配置。
#[tauri::command]
pub fn add_game_dir(state: State<'_, AppState>, path: String) -> CmdResult<Config> {
    let path = path.trim().to_string();
    if path.is_empty() {
        return Err("目录不能为空".into());
    }
    let mut cfg = state.config.lock().unwrap().clone();
    std::fs::create_dir_all(&path).map_err(err)?;
    cfg.set_active_dir(&path);
    config::save(&cfg).map_err(err)?;
    *state.config.lock().unwrap() = cfg.clone();
    Ok(cfg)
}

#[tauri::command]
pub fn switch_game_dir(state: State<'_, AppState>, path: String) -> CmdResult<Config> {
    let mut cfg = state.config.lock().unwrap().clone();
    let exists = cfg
        .game_dirs
        .iter()
        .any(|entry| entry.path.eq_ignore_ascii_case(&path));
    if !exists {
        return Err("目录不在列表中".into());
    }
    cfg.set_active_dir(&path);
    config::save(&cfg).map_err(err)?;
    *state.config.lock().unwrap() = cfg.clone();
    Ok(cfg)
}

#[tauri::command]
pub fn remove_game_dir(state: State<'_, AppState>, path: String) -> CmdResult<Config> {
    let mut cfg = state.config.lock().unwrap().clone();
    cfg.game_dirs
        .retain(|entry| !entry.path.eq_ignore_ascii_case(&path));
    if cfg.game_dirs.is_empty() {
        return Err("至少保留一个游戏目录".into());
    }
    if cfg.game_dir.eq_ignore_ascii_case(&path) {
        let next = cfg.game_dirs[0].path.clone();
        cfg.set_active_dir(&next);
    }
    cfg.sync_game_dirs();
    config::save(&cfg).map_err(err)?;
    *state.config.lock().unwrap() = cfg.clone();
    Ok(cfg)
}

#[tauri::command]
pub async fn fetch_versions(
    state: State<'_, AppState>,
    force: bool,
) -> CmdResult<Vec<VersionSummary>> {
    let cfg = state.config.lock().unwrap().clone();
    let manifest = if !force {
        state.manifest.lock().unwrap().clone()
    } else {
        None
    };
    let manifest = match manifest {
        Some(m) => m,
        None => {
            let m = install::fetch_manifest(&state.client, cfg.use_mirror())
                .await
                .map_err(err)?;
            *state.manifest.lock().unwrap() = Some(m.clone());
            m
        }
    };
    let installed: std::collections::HashSet<String> =
        install::list_installed(&cfg).into_iter().map(|v| v.id).collect();
    Ok(manifest
        .versions
        .into_iter()
        .map(|v| VersionSummary {
            installed: installed.contains(&v.id),
            id: v.id,
            kind: v.kind,
            release_time: v.release_time,
        })
        .collect())
}

#[tauri::command]
pub fn installed_versions(state: State<'_, AppState>) -> Vec<InstalledVersion> {
    let cfg = state.config.lock().unwrap().clone();
    install::list_installed(&cfg)
}

#[tauri::command]
pub async fn install_version(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
) -> CmdResult<InstalledVersion> {
    let cfg = state.config.lock().unwrap().clone();
    {
        let guard = state.progress.lock().unwrap();
        if let Some(p) = guard.as_ref() {
            if p.active.load(Ordering::Relaxed) {
                return Err("已有下载任务正在进行".into());
            }
        }
    }
    let progress = ProgressState::new("install");
    progress.set_message(format!("准备安装 {version_id}…"));
    *state.progress.lock().unwrap() = Some(progress.clone());

    let result = install::install_version(&app, &state.client, &cfg, &version_id, progress.clone())
        .await;

    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(&app, &progress);
    *state.progress.lock().unwrap() = None;

    match result {
        Ok(version) => {
            *state.config.lock().unwrap() = {
                let mut c = cfg;
                c.selected_version = Some(version_id);
                let _ = config::save(&c);
                c
            };
            Ok(version)
        }
        Err(e) => Err(err(e)),
    }
}

#[tauri::command]
pub async fn loader_options(
    state: State<'_, AppState>,
    version_id: String,
) -> CmdResult<Vec<LoaderOption>> {
    let cfg = state.config.lock().unwrap().clone();
    Ok(install::loader_options(&state.client, &cfg, &version_id).await)
}

#[tauri::command]
pub async fn loader_versions(
    state: State<'_, AppState>,
    version_id: String,
    loader: String,
) -> CmdResult<Vec<install::LoaderVersion>> {
    let cfg = state.config.lock().unwrap().clone();
    install::loader_versions(&state.client, &cfg, &version_id, &loader)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn install_loader(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
    loader: String,
    loader_version: Option<String>,
) -> CmdResult<InstalledVersion> {
    let cfg = state.config.lock().unwrap().clone();
    {
        let guard = state.progress.lock().unwrap();
        if let Some(p) = guard.as_ref() {
            if p.active.load(Ordering::Relaxed) {
                return Err("已有下载任务正在进行".into());
            }
        }
    }
    let progress = ProgressState::new("install");
    progress.set_message(format!("准备安装 {loader}…"));
    *state.progress.lock().unwrap() = Some(progress.clone());

    let result = install::install_loader(
        &app,
        &state.client,
        &cfg,
        &version_id,
        &loader,
        loader_version.as_deref(),
        progress.clone(),
    )
    .await;

    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(&app, &progress);
    *state.progress.lock().unwrap() = None;

    match result {
        Ok(version) => {
            let mut current = cfg;
            current.selected_version = Some(version.id.clone());
            let _ = config::save(&current);
            *state.config.lock().unwrap() = current;
            Ok(version)
        }
        Err(e) => Err(err(e)),
    }
}

#[tauri::command]
pub fn cancel_install(state: State<'_, AppState>) {
    if let Some(p) = state.progress.lock().unwrap().as_ref() {
        p.cancel.store(true, Ordering::Relaxed);
        p.active.store(false, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn delete_version(state: State<'_, AppState>, version_id: String) -> CmdResult<()> {
    let cfg = state.config.lock().unwrap().clone();
    let dir = install::resolve_version_dir(&cfg, &version_id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(err)?;
    }
    let _ = instance::remove(&cfg, &version_id);
    Ok(())
}

#[tauri::command]
pub fn instance_info(
    state: State<'_, AppState>,
    version_id: String,
) -> CmdResult<InstanceInfo> {
    let cfg = state.config.lock().unwrap().clone();
    instance::info(&cfg, &version_id).map_err(err)
}

#[tauri::command]
pub fn instance_metas(
    state: State<'_, AppState>,
) -> std::collections::HashMap<String, InstanceMeta> {
    let cfg = state.config.lock().unwrap().clone();
    instance::all(&cfg)
}

#[tauri::command]
pub fn instance_save_meta(
    state: State<'_, AppState>,
    version_id: String,
    meta: InstanceMeta,
) -> CmdResult<InstanceInfo> {
    let cfg = state.config.lock().unwrap().clone();
    instance::save(&cfg, &version_id, &meta).map_err(err)?;
    instance::info(&cfg, &version_id).map_err(err)
}

#[tauri::command]
pub fn instance_files(
    state: State<'_, AppState>,
    version_id: String,
    kind: String,
) -> CmdResult<Vec<InstanceFile>> {
    let cfg = state.config.lock().unwrap().clone();
    instance::list_files(&cfg, &version_id, &kind).map_err(err)
}

#[tauri::command]
pub fn instance_toggle_file(
    state: State<'_, AppState>,
    version_id: String,
    kind: String,
    name: String,
) -> CmdResult<()> {
    let cfg = state.config.lock().unwrap().clone();
    instance::toggle_file(&cfg, &version_id, &kind, &name).map_err(err)
}

#[tauri::command]
pub fn instance_delete_file(
    state: State<'_, AppState>,
    version_id: String,
    kind: String,
    name: String,
) -> CmdResult<()> {
    let cfg = state.config.lock().unwrap().clone();
    instance::delete_file(&cfg, &version_id, &kind, &name).map_err(err)
}

#[tauri::command]
pub fn instance_import_files(
    state: State<'_, AppState>,
    version_id: String,
    kind: String,
    paths: Vec<String>,
) -> CmdResult<u32> {
    let cfg = state.config.lock().unwrap().clone();
    instance::import_paths(&cfg, &version_id, &kind, &paths).map_err(err)
}

#[tauri::command]
pub fn instance_paths(
    state: State<'_, AppState>,
    version_id: String,
) -> InstancePaths {
    let cfg = state.config.lock().unwrap().clone();
    instance::paths(&cfg, &version_id)
}

#[tauri::command]
pub fn modpack_inspect(path: String) -> CmdResult<modpack::PackPreview> {
    modpack::inspect(&path).map_err(err)
}

#[tauri::command]
pub async fn modpack_import(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    options: modpack::ImportOptions,
) -> CmdResult<modpack::ImportResult> {
    let cfg = state.config.lock().unwrap().clone();
    {
        let guard = state.progress.lock().unwrap();
        if let Some(progress) = guard.as_ref() {
            if progress.active.load(Ordering::Relaxed) {
                return Err("已有下载任务正在进行".into());
            }
        }
    }
    let progress = ProgressState::new("install");
    progress.set_message("准备导入整合包…");
    *state.progress.lock().unwrap() = Some(progress.clone());

    let result = modpack::import(&app, &state.client, &cfg, &path, &options, progress.clone()).await;

    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(&app, &progress);
    *state.progress.lock().unwrap() = None;

    match result {
        Ok(imported) => {
            let mut current = cfg;
            current.selected_version = Some(imported.instance_id.clone());
            let _ = config::save(&current);
            *state.config.lock().unwrap() = current;
            Ok(imported)
        }
        Err(error) => Err(err(error)),
    }
}

#[tauri::command]
pub fn modpack_export_preview(
    state: State<'_, AppState>,
    version_id: String,
) -> CmdResult<modpack::ExportPreview> {
    let cfg = state.config.lock().unwrap().clone();
    modpack::export_preview(&cfg, &version_id).map_err(err)
}

#[tauri::command]
pub async fn modpack_export(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
    options: modpack::ExportOptions,
) -> CmdResult<modpack::ExportResult> {
    let cfg = state.config.lock().unwrap().clone();
    {
        let guard = state.progress.lock().unwrap();
        if let Some(progress) = guard.as_ref() {
            if progress.active.load(Ordering::Relaxed) {
                return Err("已有任务正在进行".into());
            }
        }
    }
    let progress = ProgressState::new("install");
    progress.set_message("准备导出整合包…");
    *state.progress.lock().unwrap() = Some(progress.clone());

    let result = modpack::export(&app, &cfg, &version_id, &options, progress.clone()).await;

    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(&app, &progress);
    *state.progress.lock().unwrap() = None;

    result.map_err(err)
}

#[derive(serde::Serialize)]
pub struct InstanceContext {
    pub game_version: String,
    pub loader: String,
}

#[tauri::command]
pub fn instance_context(
    state: State<'_, AppState>,
    version_id: String,
) -> CmdResult<InstanceContext> {
    let cfg = state.config.lock().unwrap().clone();
    let (game_version, loader) =
        install::instance_context(&cfg, &version_id).map_err(err)?;
    Ok(InstanceContext {
        game_version,
        loader,
    })
}

#[tauri::command]
pub async fn cf_search(
    state: State<'_, AppState>,
    class_id: i64,
    query: String,
    game_version: Option<String>,
    loader: Option<String>,
    index: u32,
    sort_field: Option<u32>,
) -> CmdResult<curseforge::CfSearchResult> {
    curseforge::search(
        &state.client,
        class_id,
        &query,
        game_version.as_deref(),
        loader.as_deref(),
        index,
        20,
        sort_field.unwrap_or(2),
    )
    .await
    .map_err(err)
}

#[tauri::command]
pub async fn cf_game_versions(
    state: State<'_, AppState>,
) -> CmdResult<Vec<String>> {
    curseforge::game_versions(&state.client).await.map_err(err)
}

#[tauri::command]
pub async fn cf_files(
    state: State<'_, AppState>,
    mod_id: String,
    game_version: Option<String>,
    loader: Option<String>,
) -> CmdResult<Vec<curseforge::CfFileView>> {
    let id: i64 = mod_id
        .parse()
        .map_err(|_| "无效的 CurseForge 项目 ID".to_string())?;
    curseforge::files(
        &state.client,
        id,
        game_version.as_deref(),
        loader.as_deref(),
    )
    .await
    .map_err(err)
}

#[tauri::command]
pub async fn modrinth_search(
    state: State<'_, AppState>,
    kind: String,
    query: String,
    game_version: Option<String>,
    loader: Option<String>,
    index: u32,
    sort_field: Option<u32>,
) -> CmdResult<curseforge::CfSearchResult> {
    crate::modrinth::search(
        &state.client,
        &kind,
        &query,
        game_version.as_deref(),
        loader.as_deref(),
        index,
        20,
        sort_field.unwrap_or(2),
    )
    .await
    .map_err(err)
}

#[tauri::command]
pub async fn modrinth_files(
    state: State<'_, AppState>,
    project_id: String,
    game_version: Option<String>,
    loader: Option<String>,
) -> CmdResult<Vec<curseforge::CfFileView>> {
    crate::modrinth::files(
        &state.client,
        &project_id,
        game_version.as_deref(),
        loader.as_deref(),
    )
    .await
    .map_err(err)
}

#[tauri::command]
pub async fn cf_install_file(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
    kind: String,
    url: String,
    file_name: String,
) -> CmdResult<String> {
    let cfg = state.config.lock().unwrap().clone();
    {
        let guard = state.progress.lock().unwrap();
        if let Some(p) = guard.as_ref() {
            if p.active.load(Ordering::Relaxed) {
                return Err("已有下载任务正在进行".into());
            }
        }
    }
    let progress = ProgressState::new("install");
    progress.set_message(format!("准备下载 {file_name}…"));
    *state.progress.lock().unwrap() = Some(progress.clone());

    let result = curseforge::install_file(
        &app,
        &state.client,
        &cfg,
        &version_id,
        &kind,
        &url,
        &file_name,
        progress.clone(),
    )
    .await;

    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(&app, &progress);
    *state.progress.lock().unwrap() = None;

    result.map_err(err)
}

#[tauri::command]
pub fn default_skin() -> String {
    skins::default_skin()
}

#[tauri::command]
pub fn get_accounts(state: State<'_, AppState>) -> Vec<AccountView> {
    // 每次从磁盘重新读取，确保微软登录完成后能立即看到新账号
    let accounts = auth::load_accounts();
    *state.accounts.lock().unwrap() = accounts.clone();
    accounts.iter().map(AccountView::from).collect()
}

#[tauri::command]
pub fn create_offline_account(
    state: State<'_, AppState>,
    name: String,
) -> CmdResult<AccountView> {
    let account = auth::create_offline(&name).map_err(err)?;
    {
        let mut accounts = state.accounts.lock().unwrap();
        accounts.retain(|a| a.id != account.id);
        accounts.push(account.clone());
        auth::save_accounts(&accounts).map_err(err)?;
    }
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.selected_account = Some(account.id.clone());
        config::save(&cfg).map_err(err)?;
    }
    Ok(AccountView::from(&account))
}

#[tauri::command]
pub fn remove_account(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let mut accounts = state.accounts.lock().unwrap();
    accounts.retain(|a| a.id != id);
    auth::save_accounts(&accounts).map_err(err)?;
    let mut cfg = state.config.lock().unwrap();
    if cfg.selected_account.as_deref() == Some(id.as_str()) {
        cfg.selected_account = accounts.first().map(|a| a.id.clone());
        let _ = config::save(&cfg);
    }
    Ok(())
}

#[tauri::command]
pub fn select_account(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.selected_account = Some(id);
    config::save(&cfg).map_err(err)
}

#[derive(serde::Serialize)]
pub struct AccountSkinView {
    pub data_url: String,
    pub model: String,
    pub source: String,
}

fn find_account(account_id: &str) -> CmdResult<Account> {
    auth::load_accounts()
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "账号不存在".into())
}

#[tauri::command]
pub async fn get_account_skin(
    state: State<'_, AppState>,
    account_id: String,
) -> CmdResult<AccountSkinView> {
    let account = find_account(&account_id)?;
    let (data_url, model, source) = skins::load_skin(&state.client, &account).await;
    Ok(AccountSkinView {
        data_url,
        model,
        source,
    })
}

#[tauri::command]
pub fn save_account_skin(
    state: State<'_, AppState>,
    account_id: String,
    png_base64: String,
    model: String,
) -> CmdResult<AccountView> {
    let path = skins::save_local(&account_id, &png_base64).map_err(err)?;
    let mut accounts = auth::load_accounts();
    let account = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "账号不存在".to_string())?;
    account.skin_path = Some(path.to_string_lossy().to_string());
    account.skin_model = model;
    let view = AccountView::from(&*account);
    auth::save_accounts(&accounts).map_err(err)?;
    *state.accounts.lock().unwrap() = accounts;
    Ok(view)
}

#[tauri::command]
pub fn clear_account_skin(
    state: State<'_, AppState>,
    account_id: String,
) -> CmdResult<AccountView> {
    let mut accounts = auth::load_accounts();
    let account = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "账号不存在".to_string())?;
    if let Some(path) = account.skin_path.take() {
        let _ = std::fs::remove_file(path);
    }
    account.skin_model = "classic".into();
    let view = AccountView::from(&*account);
    auth::save_accounts(&accounts).map_err(err)?;
    *state.accounts.lock().unwrap() = accounts;
    Ok(view)
}

#[tauri::command]
pub fn export_account_skin(account_id: String, target_path: String) -> CmdResult<()> {
    let account = find_account(&account_id)?;
    skins::export_bytes(&account, &target_path).map_err(err)
}

#[tauri::command]
pub async fn upload_account_skin(
    state: State<'_, AppState>,
    account_id: String,
    png_base64: String,
    model: String,
) -> CmdResult<AccountView> {
    let mut account = find_account(&account_id)?;
    skins::upload_skin(&state.client, &account, &png_base64, &model)
        .await
        .map_err(err)?;
    let path = skins::save_local(&account_id, &png_base64).map_err(err)?;
    account.skin_path = Some(path.to_string_lossy().to_string());
    account.skin_model = model;
    let mut accounts = auth::load_accounts();
    if let Some(slot) = accounts.iter_mut().find(|a| a.id == account_id) {
        *slot = account.clone();
    }
    auth::save_accounts(&accounts).map_err(err)?;
    *state.accounts.lock().unwrap() = accounts;
    Ok(AccountView::from(&account))
}

#[tauri::command]
pub async fn ms_login_begin(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<DeviceCode> {
    let code = auth::begin_device_code(&state.client).await.map_err(err)?;
    let client = state.client.clone();
    let app_clone = app.clone();
    let code_clone = code.clone();
    tokio::spawn(async move {
        match auth::poll_device_login(app_clone.clone(), client, code_clone).await {
            Ok(account) => {
                // 由前端刷新账号列表
                let _ = app_clone.emit("ms-login-account", AccountView::from(&account));
            }
            Err(e) => {
                let _ = app_clone.emit(
                    "ms-login",
                    serde_json::json!({ "ok": false, "message": e.to_string() }),
                );
            }
        }
    });
    Ok(code)
}

#[tauri::command]
pub fn detect_java(state: State<'_, AppState>) -> Vec<JavaInfo> {
    let cfg = state.config.lock().unwrap().clone();
    java::detect(&cfg)
}

#[tauri::command]
pub async fn install_java(
    app: AppHandle,
    state: State<'_, AppState>,
    major: u32,
) -> CmdResult<JavaInfo> {
    let cfg = state.config.lock().unwrap().clone();
    let progress = ProgressState::new("java");
    progress.set_message(format!("准备安装 Java {major}…"));
    *state.progress.lock().unwrap() = Some(progress.clone());

    let result = java::install_runtime(&app, &cfg, major, progress.clone()).await;
    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(&app, &progress);
    *state.progress.lock().unwrap() = None;

    match result {
        Ok(info) => {
            let mut cfg = state.config.lock().unwrap();
            cfg.java_path = Some(info.path.clone());
            let _ = config::save(&cfg);
            Ok(info)
        }
        Err(e) => Err(err(e)),
    }
}

#[tauri::command]
pub async fn launch_version(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
) -> CmdResult<u32> {
    // 版本级启动设置覆盖全局配置
    let cfg = instance::apply(&state.config.lock().unwrap().clone(), &version_id);
    let accounts = state.accounts.lock().unwrap().clone();
    let selected = cfg
        .selected_account
        .as_ref()
        .and_then(|id| accounts.iter().find(|a| &a.id == id).cloned())
        .or_else(|| accounts.first().cloned())
        .unwrap_or_else(|| auth::create_offline("Steve").unwrap());

    let mut account: Account = selected;
    if account.kind == "microsoft" {
        match auth::refresh(&state.client, &account).await {
            Ok(refreshed) => {
                account = refreshed;
                let mut list = state.accounts.lock().unwrap();
                if let Some(slot) = list.iter_mut().find(|a| a.id == account.id) {
                    *slot = account.clone();
                } else {
                    list.push(account.clone());
                }
                let _ = auth::save_accounts(&list);
            }
            Err(e) => {
                let _ = app.emit(
                    "game-log",
                    launch::LogLine {
                        stream: "system".into(),
                        line: format!("账号刷新失败：{e}"),
                    },
                );
            }
        }
    }

    let pid = launch::spawn(&app, &cfg, &version_id, &account).map_err(err)?;
    *state.child_pid.lock().unwrap() = Some(pid);
    Ok(pid)
}

#[tauri::command]
pub fn kill_game(state: State<'_, AppState>) -> CmdResult<()> {
    if let Some(pid) = *state.child_pid.lock().unwrap() {
        launch::kill(pid).map_err(err)?;
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct BackgroundInfo {
    pub path: String,
    pub data_url: String,
}

fn image_mime(path: &std::path::Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}

fn encode_background(path: &std::path::Path) -> CmdResult<String> {
    let meta = std::fs::metadata(path).map_err(err)?;
    if meta.len() > 24 * 1024 * 1024 {
        return Err("背景图片过大（上限 24 MB）".into());
    }
    let mime = image_mime(path).ok_or_else(|| "仅支持 PNG / JPG / WEBP / BMP / GIF 图片".to_string())?;
    let bytes = std::fs::read(path).map_err(err)?;
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

#[tauri::command]
pub fn import_background(path: String) -> CmdResult<BackgroundInfo> {
    let source = std::path::PathBuf::from(&path);
    encode_background(&source)?;
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();
    let dir = crate::config::data_dir();
    std::fs::create_dir_all(&dir).map_err(err)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let target = dir.join(format!("background-{stamp}.{ext}"));
    std::fs::copy(&source, &target).map_err(err)?;
    let data_url = encode_background(&target)?;
    Ok(BackgroundInfo {
        path: target.to_string_lossy().to_string(),
        data_url,
    })
}

#[tauri::command]
pub fn background_data_url(path: String) -> CmdResult<String> {
    encode_background(std::path::Path::new(&path))
}
