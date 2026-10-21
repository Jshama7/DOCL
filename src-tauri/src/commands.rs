use crate::auth::{self, Account, AccountView, DeviceCode};
use crate::config::{self, Config};
use crate::download::{self, ProgressState};
use crate::install::{self, InstalledVersion};
use crate::java::{self, JavaInfo};
use crate::launch;
use crate::models::*;
use crate::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> CmdResult<Config> {
    config::save(&config).map_err(err)?;
    std::fs::create_dir_all(&config.game_dir).map_err(err)?;
    *state.config.lock().unwrap() = config.clone();
    Ok(config)
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
    Ok(())
}

#[tauri::command]
pub fn get_accounts(state: State<'_, AppState>) -> Vec<AccountView> {
    state
        .accounts
        .lock()
        .unwrap()
        .iter()
        .map(AccountView::from)
        .collect()
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
    let cfg = state.config.lock().unwrap().clone();
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
