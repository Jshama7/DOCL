use crate::auth::Account;
use crate::config::Config;
use crate::install;
use crate::java;
use crate::models::*;
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
pub struct LogLine {
    pub stream: String,
    pub line: String,
}

#[derive(Serialize, Clone)]
pub struct GameExit {
    pub code: Option<i32>,
    pub pid: u32,
}

fn arg_values(list: &[ArgValue]) -> Vec<String> {
    let mut result = Vec::new();
    for arg in list {
        match arg {
            ArgValue::Plain(s) => result.push(s.clone()),
            ArgValue::Conditional { rules, value } => {
                if install::rules_allow(Some(rules)) {
                    match value {
                        RawArgValue::One(s) => result.push(s.clone()),
                        RawArgValue::Many(v) => result.extend(v.iter().cloned()),
                    }
                }
            }
        }
    }
    result
}

fn replace(template: &str, vars: &std::collections::HashMap<&str, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("${{{key}}}"), value);
    }
    out
}

pub fn find_java(cfg: &Config, required: u32) -> Result<java::JavaInfo> {
    if let Some(path) = &cfg.java_path {
        if let Some(info) = java::inspect_java(std::path::Path::new(path), "手动指定") {
            return Ok(info);
        }
    }
    let detected = java::detect(cfg);
    if let Some(info) = java::pick_for_major(&detected, required) {
        return Ok(info);
    }
    Err(anyhow!(
        "LAUNCH_NO_JAVA:当前版本需要 Java {required}，本机未找到可用运行时"
    ))
}

pub fn build(
    cfg: &Config,
    merged: &install::MergedDetail,
    version_id: &str,
    account: &Account,
) -> Result<(PathBuf, Vec<String>, Vec<String>)> {
    let detail = &merged.detail;
    let game_root = cfg.game_root();
    let game_dir = cfg.launch_dir(version_id);
    std::fs::create_dir_all(&game_dir)?;

    let natives_dir = install::natives_dir(cfg, version_id);
    if !natives_dir.exists() {
        bail!("本地库尚未解压，请重新安装该版本");
    }

    let required = detail.required_java();
    let java = find_java(cfg, required)?;

    let classpath_entries =
        install::merged_classpath_entries(cfg, detail, merged.jar_id.as_deref());
    let classpath = classpath_entries
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");

    let mut vars: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    vars.insert("natives_directory", natives_dir.to_string_lossy().to_string());
    vars.insert("launcher_name", "DOCL".into());
    vars.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    vars.insert("classpath", classpath.clone());
    vars.insert("classpath_separator", ";".into());
    vars.insert("auth_player_name", account.name.clone());
    vars.insert("version_name", detail.id.clone());
    vars.insert(
        "game_directory",
        game_dir.to_string_lossy().to_string(),
    );
    vars.insert(
        "assets_root",
        game_root.join("assets").to_string_lossy().to_string(),
    );
    vars.insert(
        "assets_index_name",
        detail
            .asset_index
            .as_ref()
            .map(|a| a.id.clone())
            .or_else(|| detail.assets.clone())
            .unwrap_or_else(|| "legacy".into()),
    );
    vars.insert("auth_uuid", account.uuid.clone());
    vars.insert("auth_access_token", account.access_token.clone());
    vars.insert("auth_session", account.access_token.clone());
    vars.insert(
        "user_type",
        if account.kind == "microsoft" {
            "msa".into()
        } else {
            "legacy".into()
        },
    );
    vars.insert(
        "version_type",
        if detail.kind.is_empty() {
            "release".into()
        } else {
            detail.kind.clone()
        },
    );
    vars.insert("user_properties", "{}".into());
    vars.insert("clientid", String::new());
    vars.insert("auth_xuid", String::new());
    vars.insert("resolution_width", cfg.window_width.to_string());
    vars.insert("resolution_height", cfg.window_height.to_string());
    vars.insert("library_directory", game_root.join("libraries").to_string_lossy().to_string());

    let mut jvm_args: Vec<String> = Vec::new();
    if let Some(args) = &detail.arguments {
        for value in arg_values(&args.jvm) {
            jvm_args.push(replace(&value, &vars));
        }
    }
    if let Some(logging) = detail.logging.as_ref().and_then(|l| l.client.as_ref()) {
        let file_name = logging
            .file
            .url
            .rsplit('/')
            .next()
            .unwrap_or("log4j.xml")
            .to_string();
        let log_path = game_root.join("assets").join("log_configs").join(file_name);
        if log_path.exists() {
            vars.insert("path", log_path.to_string_lossy().to_string());
            jvm_args.push(replace(&logging.argument, &vars));
        }
    }

    let mut combined_jvm: Vec<String> = vec![
        format!("-Xmx{}M", cfg.memory_mb.max(512)),
        format!("-Xms{}M", (cfg.memory_mb / 2).clamp(256, 2048)),
        "-XX:+UseG1GC".into(),
        "-Dfile.encoding=UTF-8".into(),
        "-Dstdout.encoding=UTF-8".into(),
        "-Dstderr.encoding=UTF-8".into(),
    ];
    combined_jvm.extend(jvm_args);
    for custom in cfg.java_args.split_whitespace() {
        if !custom.trim().is_empty() {
            combined_jvm.push(custom.to_string());
        }
    }
    if !combined_jvm.iter().any(|a| a.starts_with("-Djava.library.path")) {
        combined_jvm.push(format!(
            "-Djava.library.path={}",
            natives_dir.to_string_lossy()
        ));
    }
    if !combined_jvm.iter().any(|a| a == "-cp" || a == "-classpath") {
        combined_jvm.push("-cp".into());
        combined_jvm.push(classpath);
    }

    let mut game_args: Vec<String> = Vec::new();
    if let Some(args) = &detail.arguments {
        for value in arg_values(&args.game) {
            game_args.push(replace(&value, &vars));
        }
    } else if let Some(legacy) = &detail.minecraft_arguments {
        for value in legacy.split_whitespace() {
            game_args.push(replace(value, &vars));
        }
    }

    let mut command_args = combined_jvm;
    let main_class = detail
        .main_class
        .clone()
        .unwrap_or_else(|| "net.minecraft.client.main.Main".into());
    command_args.push(main_class);
    command_args.extend(game_args);

    Ok((PathBuf::from(java.path), command_args, Vec::new()))
}

pub fn spawn(
    app: &AppHandle,
    cfg: &Config,
    version_id: &str,
    account: &Account,
) -> Result<u32> {
    let merged = install::resolve_detail(cfg, version_id)?;
    let game_dir = cfg.launch_dir(version_id);
    let (java_path, args, _) = build(cfg, &merged, version_id, account)?;

    let _ = app.emit(
        "game-log",
        LogLine {
            stream: "system".into(),
            line: format!("使用 Java：{}", java_path.display()),
        },
    );
    let _ = app.emit(
        "game-log",
        LogLine {
            stream: "system".into(),
            line: format!(
                "启动参数：{} 个，游戏目录：{}",
                args.len(),
                game_dir.display()
            ),
        },
    );

    let mut cmd = Command::new(&java_path);
    cmd.current_dir(&game_dir)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn()?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    if let Some(stdout) = stdout {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = app.emit(
                    "game-log",
                    LogLine {
                        stream: "stdout".into(),
                        line,
                    },
                );
            }
        });
    }
    if let Some(stderr) = stderr {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = app.emit(
                    "game-log",
                    LogLine {
                        stream: "stderr".into(),
                        line,
                    },
                );
            }
        });
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let status = child.wait();
        let _ = app.emit(
            "game-exit",
            GameExit {
                code: status.ok().and_then(|s| s.code()),
                pid,
            },
        );
    });

    Ok(pid)
}

pub fn kill(pid: u32) -> Result<()> {
    let mut cmd = Command::new("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.status()?;
    Ok(())
}
