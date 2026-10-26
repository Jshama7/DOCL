use crate::config::Config;
use crate::instance;
use anyhow::Result;
use serde::Serialize;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// 单个日志文件最多读取的字节数，避免超大 debug.log 拖慢分析。
const READ_LIMIT: u64 = 3 * 1024 * 1024;
/// 崩溃报告与 latest.log 的写入时间差在这个范围内时，认为崩溃报告属于本次运行。
const SAME_RUN_WINDOW: u64 = 90;

#[derive(Debug, Clone, Serialize)]
pub struct CrashLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrashMatch {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub advice: Vec<String>,
    pub evidence: Vec<String>,
    pub links: Vec<CrashLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrashReport {
    pub version_id: String,
    /// 是否找到了可分析的日志
    pub found: bool,
    pub source: String,
    pub source_path: String,
    pub source_kind: String,
    pub modified: u64,
    pub headline: String,
    pub description: Option<String>,
    pub matches: Vec<CrashMatch>,
    pub excerpt: Vec<String>,
    pub scanned: Vec<String>,
}

struct Rule {
    id: &'static str,
    title: &'static str,
    summary: &'static str,
    patterns: &'static [&'static str],
    advice: &'static [&'static str],
    links: &'static [(&'static str, &'static str)],
}

const RULES: &[Rule] = &[
    Rule {
        id: "java-version",
        title: "Java 版本不匹配",
        summary: "当前 Java 无法加载游戏或加载器的类文件，通常是 Java 版本低于该版本的要求。",
        patterns: &[
            "unsupportedclassversionerror",
            "class file major version",
            "has been compiled by a more recent version of the java runtime",
        ],
        advice: &[
            "在「设置 → Java」里安装或选择匹配的 Java：1.20.5+ 需要 Java 21，1.17+ 需要 17，1.16 及以下用 8。",
            "版本设置里如果手动指定过 java.exe，确认它没有指向旧版。",
            "改完后重新启动游戏。",
        ],
        links: &[("下载 Temurin JDK", "https://adoptium.net/temurin/releases/")],
    },
    Rule {
        id: "out-of-memory",
        title: "内存不足",
        summary: "Java 堆内存不够，常见于大型整合包、高清材质或视距过高。",
        patterns: &[
            "java.lang.outofmemoryerror",
            "gc overhead limit exceeded",
            "java heap space",
            "could not reserve enough space",
            "unable to allocate",
        ],
        advice: &[
            "在设置或版本设置里调大内存，整合包建议 6–8 GB，但不要超过物理内存的一半。",
            "关闭浏览器等占用内存较多的程序。",
            "降低渲染距离、模拟距离或光影质量后重试。",
        ],
        links: &[],
    },
    Rule {
        id: "mod-conflict",
        title: "Mod 冲突或加载失败",
        summary: "多个 Mod 互相冲突，或加载器无法解析依赖关系。",
        patterns: &[
            "duplicate mods found",
            "duplicate mod",
            "mod resolution failed",
            "incompatible mod set",
            "mod loading has failed",
            "modloadingexception",
            "loadingfailedexception",
        ],
        advice: &[
            "先禁用最近新增的 Mod，再逐个开启，定位冲突源。",
            "检查是否装了功能重复的 Mod（例如两个优化 Mod、两个同一前置的不同版本）。",
            "在崩溃报告里搜索「Failure message」，通常会直接写出冲突的 Mod ID。",
        ],
        links: &[],
    },
    Rule {
        id: "missing-dependency",
        title: "缺少前置 Mod",
        summary: "某个 Mod 依赖的前置库没有安装，或版本不满足要求。",
        patterns: &[
            "missing or unsupported mandatory dependencies",
            "missing required dependency",
            "modresolutionexception",
            "missing dependency",
        ],
        advice: &[
            "按日志里列出的 modid 安装对应前置。",
            "确认前置 Mod 的版本与游戏版本、加载器版本一致。",
            "整合包建议使用导入功能完整安装，避免手动拷贝漏掉依赖。",
        ],
        links: &[],
    },
    Rule {
        id: "mixin",
        title: "Mixin 注入失败",
        summary: "某个 Mod 的 Mixin 与当前游戏版本或其他 Mod 不兼容。",
        patterns: &[
            "mixin apply failed",
            "mixintransformererror",
            "mixin.injection",
            "mixin config",
        ],
        advice: &[
            "确认该 Mod 支持当前游戏版本和加载器版本。",
            "尝试更新该 Mod 或它依赖的前置库。",
            "在崩溃报告里找到「Mixin apply failed」后面的 Mod 名称并暂时禁用。",
        ],
        links: &[],
    },
    Rule {
        id: "graphics",
        title: "显卡驱动 / OpenGL 问题",
        summary: "游戏无法创建渲染窗口或初始化 OpenGL，通常与显卡驱动有关。",
        patterns: &[
            "glfw error",
            "failed to create window",
            "failed to create glfw window",
            "pixel format not accelerated",
            "doesn't support opengl",
            "could not create opengl context",
        ],
        advice: &[
            "到显卡厂商官网更新驱动，不要只依赖系统自带驱动。",
            "确认不是通过远程桌面或虚拟机运行游戏。",
            "在设置里打开「优先使用高性能显卡」后重试。",
        ],
        links: &[],
    },
    Rule {
        id: "corrupted-jar",
        title: "文件损坏",
        summary: "某个 jar 下载不完整或已损坏，加载时无法解压。",
        patterns: &[
            "zip end header not found",
            "invalid or corrupted jar",
            "java.util.zip.zipexception",
            "error in opening zip file",
            "invalid jar file",
        ],
        advice: &[
            "在版本设置里重新下载该版本或对应的 Mod。",
            "检查 mods 目录里是否有 0 字节或异常的 jar 并删除。",
            "确认磁盘剩余空间充足。",
        ],
        links: &[],
    },
    Rule {
        id: "class-loading",
        title: "类加载失败",
        summary: "Mod 与游戏版本不匹配，或缺少前置导致类找不到。",
        patterns: &[
            "java.lang.nosuchmethoderror",
            "java.lang.noclassdeffounderror",
            "java.lang.classnotfounderror",
            "java.lang.exceptioninitializererror",
        ],
        advice: &[
            "确认所有 Mod 都是同一游戏版本。",
            "删除只适用于其他版本的 Mod。",
            "补齐报错里出现的类所属的前置 Mod。",
        ],
        links: &[],
    },
    Rule {
        id: "jvm-fatal",
        title: "Java 虚拟机崩溃",
        summary: "JVM 层发生严重错误（SIGSEGV/SIGILL 等），常见于驱动、本地库或内存不稳定。",
        patterns: &[
            "a fatal error has been detected by the java runtime environment",
            "sigtrap",
            "sigsegv",
            "hs_err_pid",
        ],
        advice: &[
            "更新显卡驱动和 Java 版本。",
            "关闭 CPU/内存超频（包括 XMP），检查内存稳定性。",
            "在版本目录里找到 hs_err_pid*.log，崩溃模块名就在开头几行。",
        ],
        links: &[("崩溃报告说明", "https://minecraft.wiki/w/Crash_report")],
    },
    Rule {
        id: "auth",
        title: "账号验证失败",
        summary: "正版登录状态失效，或无法连接 Mojang / Microsoft 的验证服务器。",
        patterns: &[
            "failed to verify username",
            "invalid session",
            "authentication servers",
            "invalid credentials",
        ],
        advice: &[
            "在账号页重新登录微软账号。",
            "检查网络或代理是否拦截了验证域名。",
            "离线账号请使用离线模式，或关闭服务器的正版验证。",
        ],
        links: &[],
    },
    Rule {
        id: "missing-dependency",
        title: "缺少前置 Mod",
        summary: "某个 Mod 依赖的前置没有安装，或前置版本不满足要求。",
        patterns: &[
            "java.lang.noclassdeffounderror",
            "java.lang.classnotfoundexception",
            "java.lang.nosuchmethoderror",
            "java.lang.nosuchfielderror",
            "requires any version of",
            "requires version",
            "mod file is missing",
            "missing or unsupported mandatory dependencies",
        ],
        advice: &[
            "在模组列表里搜索报错中提到的 Mod ID，安装对应的前置。",
            "在本启动器的「下载 → 模组」里打开该 Mod 详情，前置区会列出必需依赖。",
            "前置必须是同一加载器、同一 Minecraft 版本的构建。",
        ],
        links: &[],
    },
    Rule {
        id: "mixin-failure",
        title: "Mixin 注入失败",
        summary: "某个 Mod 的 Mixin 补丁无法应用，通常是 Mod 版本与游戏或加载器不匹配。",
        patterns: &[
            "mixin apply failed",
            "mixin transformation",
            "mixin.apply",
            "org.spongepowered.asm.mixin",
            "critical injection failure",
            "invalid mixin",
        ],
        advice: &[
            "把报错里出现的 Mod 更新到与当前 Minecraft 版本匹配的构建。",
            "如果同时装了多个优化 Mod（如 OptiFine 与 Sodium），只保留其中一个。",
            "在版本设置里临时禁用可疑 Mod 验证是否恢复。",
        ],
        links: &[],
    },
    Rule {
        id: "graphics-driver",
        title: "显卡驱动或 OpenGL 异常",
        summary: "创建窗口或渲染时 OpenGL 失败，多与显卡驱动、双显卡切换或远程桌面有关。",
        patterns: &[
            "glfw error",
            "opengl error",
            "pixel format",
            "wglcreatecontext",
            "failed to create window",
            "exiting current thread",
            "unsupported graphics",
        ],
        advice: &[
            "更新显卡驱动，笔记本请确认游戏跑在独显上。",
            "在设置里开启「优先使用独立显卡」，或关闭远程桌面后重试。",
            "降低光影与渲染距离，排除显存不足。",
        ],
        links: &[],
    },
    Rule {
        id: "vm-options",
        title: "Java 启动参数无效",
        summary: "JVM 不认识设置的启动参数，或无法创建虚拟机，通常是自定义参数写错或内存过大。",
        patterns: &[
            "unrecognized vm option",
            "could not create the java virtual machine",
            "error: could not create",
            "invalid maximum heap size",
            "error occurred during initialization of vm",
        ],
        advice: &[
            "在设置或版本设置里清空自定义 Java 参数后重试。",
            "检查最大内存是否超过物理内存，或是否写成了非法数值。",
            "确认使用的 Java 版本支持这些参数（部分参数仅特定版本可用）。",
        ],
        links: &[],
    },
    Rule {
        id: "mod-version-mismatch",
        title: "Mod 与游戏版本不匹配",
        summary: "Mod 声明的 Minecraft / 加载器版本和当前实例不一致。",
        patterns: &[
            "requires minecraft",
            "requires forge",
            "requires fabric",
            "requires neoforge",
            "incompatible mods found",
            "mod requires",
            "unsupported minecraft version",
        ],
        advice: &[
            "按报错提示把 Mod 换成对应 Minecraft 与加载器版本的构建。",
            "在「版本设置 → 模组」里检查最近更新过的 Mod。",
            "整合包请通过启动器完整安装，避免手动替换单个 Mod。",
        ],
        links: &[],
    },
    Rule {
        id: "world-corrupt",
        title: "存档或区块损坏",
        summary: "读取世界数据时出错，可能是区块文件损坏或存档被其他进程占用。",
        patterns: &[
            "chunk file",
            "region file",
            "exception generating new chunk",
            "corrupted",
            "world corrupt",
            "failed to load world",
            "level.dat",
        ],
        advice: &[
            "先备份 saves 目录下的存档，再尝试进入。",
            "删除该世界的 session.lock 后重试（确保没有其他客户端开着它）。",
            "如果只有特定区域崩溃，可用区块修复工具处理，或从备份恢复。",
        ],
        links: &[],
    },
    Rule {
        id: "server-port",
        title: "端口被占用",
        summary: "服务端启动时无法绑定端口，通常是已有实例或其他程序占用了同一端口。",
        patterns: &[
            "address already in use",
            "failed to bind to port",
            "bindexception",
        ],
        advice: &[
            "在联机页换一个端口启动，或先关闭已在运行的服务端。",
            "用 netstat 找到占用端口的程序后结束它。",
            "同一台机器同时开多个服务端时必须使用不同端口。",
        ],
        links: &[],
    },
    Rule {
        id: "authlib-login",
        title: "第三方账号认证失败",
        summary: "皮肤站的登录凭据已过期，或游戏无法连接到该皮肤站。",
        patterns: &[
            "authlib-injector",
            "yggdrasil",
            "failed to authenticate",
        ],
        advice: &[
            "到账号页对第三方账号重新登录一次。",
            "确认皮肤站地址可用，且该服务器允许第三方认证。",
            "如果游戏内被踢并提示验证失败，说明进的是正版验证服务器。",
        ],
        links: &[],
    },
];

/// 没有命中任何具体规则、但明显是崩溃时的兜底条目。
fn generic_match() -> CrashMatch {
    CrashMatch {
        id: "unknown".into(),
        title: "游戏崩溃（未匹配到已知原因）".into(),
        summary: "已经生成崩溃报告，但没有命中启动器内置的常见问题规则。".into(),
        advice: [
            "查看下方摘录里的「Description」和第一段异常堆栈。",
            "禁用最近新增的 Mod、资源包或光影后重试。",
            "向社区求助时附上游戏版本、加载器版本和完整崩溃报告。",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect(),
        evidence: Vec::new(),
        links: vec![CrashLink {
            label: "崩溃报告说明".into(),
            url: "https://minecraft.wiki/w/Crash_report".into(),
        }],
    }
}

fn evidence_for(text: &str, patterns: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lowered = trimmed.to_lowercase();
        if !patterns.iter().any(|pattern| lowered.contains(pattern)) {
            continue;
        }
        let clipped: String = trimmed.chars().take(240).collect();
        if !out.contains(&clipped) {
            out.push(clipped);
        }
        if out.len() >= 4 {
            break;
        }
    }
    out
}

fn description_of(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("Description:")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn matches_in(text: &str) -> Vec<CrashMatch> {
    let lowered = text.to_lowercase();
    let mut result = Vec::new();
    for rule in RULES {
        if !rule.patterns.iter().any(|pattern| lowered.contains(pattern)) {
            continue;
        }
        result.push(CrashMatch {
            id: rule.id.into(),
            title: rule.title.into(),
            summary: rule.summary.into(),
            advice: rule
                .advice
                .iter()
                .map(|value| value.to_string())
                .collect(),
            evidence: evidence_for(text, rule.patterns),
            links: rule
                .links
                .iter()
                .map(|(label, url)| CrashLink {
                    label: (*label).into(),
                    url: (*url).into(),
                })
                .collect(),
        });
        if result.len() >= 4 {
            break;
        }
    }

    let looks_like_crash = lowered.contains("---- minecraft crash report ----")
        || lowered.contains("the game crashed whilst")
        || lowered.contains("a fatal error has been detected by the java runtime environment");
    if result.is_empty() && looks_like_crash {
        result.push(generic_match());
    }
    result
}

fn modified_secs(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn read_text(path: &Path) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let mut buffer = Vec::new();
    if file.by_ref().take(READ_LIMIT).read_to_end(&mut buffer).is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&buffer).to_string()
}

fn newest_file(dir: &Path, accept: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<(u64, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !accept(&name) {
            continue;
        }
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let modified = modified_secs(&path);
        if best.as_ref().map(|(time, _)| modified > *time).unwrap_or(true) {
            best = Some((modified, path));
        }
    }
    best.map(|(_, path)| path)
}

fn excerpt_from(text: &str, kind: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    let picked: Vec<String> = if kind == "latest-log" {
        lines
            .iter()
            .rev()
            .take(60)
            .rev()
            .map(|line| line.chars().take(240).collect())
            .collect()
    } else {
        lines
            .iter()
            .take(45)
            .map(|line| line.chars().take(240).collect())
            .collect()
    };
    picked
}

pub fn analyze(cfg: &Config, version_id: &str) -> Result<CrashReport> {
    let game_dir = instance::game_dir(cfg, version_id);
    let latest_log = game_dir.join("logs").join("latest.log");
    let crash_report = newest_file(&game_dir.join("crash-reports"), |name| {
        name.starts_with("crash-") && name.ends_with(".txt")
    });
    let hs_err = newest_file(&game_dir, |name| {
        name.starts_with("hs_err_pid") && name.ends_with(".log")
    });

    let latest_modified = modified_secs(&latest_log);
    let crash_fresh = crash_report
        .as_ref()
        .map(|path| {
            let modified = modified_secs(path);
            modified + SAME_RUN_WINDOW >= latest_modified
        })
        .unwrap_or(false);
    let hs_err_fresh = hs_err
        .as_ref()
        .map(|path| {
            let modified = modified_secs(path);
            modified + SAME_RUN_WINDOW >= latest_modified
        })
        .unwrap_or(false);

    let primary = if crash_fresh {
        crash_report.clone()
    } else if hs_err_fresh {
        hs_err.clone()
    } else if latest_log.exists() {
        Some(latest_log.clone())
    } else {
        None
    };

    let mut scanned = Vec::new();
    for (path, label) in [
        (crash_report.as_ref(), "crash-report"),
        (hs_err.as_ref(), "hs-err"),
        (Some(&latest_log), "latest-log"),
    ] {
        if let Some(path) = path {
            if path.exists() {
                scanned.push(format!("{} · {}", label, path.display()));
            }
        }
    }

    let Some(primary) = primary else {
        return Ok(CrashReport {
            version_id: version_id.into(),
            found: false,
            source: String::new(),
            source_path: String::new(),
            source_kind: "none".into(),
            modified: 0,
            headline: "还没有可分析的日志".into(),
            description: None,
            matches: Vec::new(),
            excerpt: Vec::new(),
            scanned,
        });
    };

    let source_kind = if Some(&primary) == crash_report.as_ref() {
        "crash-report"
    } else if Some(&primary) == hs_err.as_ref() {
        "hs-err"
    } else {
        "latest-log"
    };

    let text = read_text(&primary);
    let mut combined = text.clone();
    if let Ok(extra) = fs::read_to_string(&latest_log) {
        combined.push_str("\n");
        combined.push_str(&extra);
    }

    let description = description_of(&text);
    let matches = matches_in(&combined);
    let headline = if source_kind == "crash-report" {
        description
            .as_ref()
            .map(|value| format!("崩溃原因：{value}"))
            .unwrap_or_else(|| "检测到崩溃报告".into())
    } else if source_kind == "hs-err" {
        "检测到 Java 虚拟机崩溃日志".into()
    } else if matches.is_empty() {
        "日志里没有发现明显错误".into()
    } else {
        "日志里发现可疑错误".into()
    };

    Ok(CrashReport {
        version_id: version_id.into(),
        found: true,
        source: primary
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default(),
        source_path: primary.to_string_lossy().to_string(),
        source_kind: source_kind.into(),
        modified: modified_secs(&primary),
        headline,
        description,
        matches,
        excerpt: excerpt_from(&text, source_kind),
        scanned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_java_version_mismatch() {
        let text = "java.lang.UnsupportedClassVersionError: net/fabricmc/loader/api/FabricLoader has been compiled by a more recent version of the Java Runtime (class file version 65.0)";
        let matches = matches_in(text);
        assert_eq!(matches[0].id, "java-version");
        assert!(!matches[0].evidence.is_empty());
    }

    #[test]
    fn detects_memory_and_graphic_issue() {
        let text = "java.lang.OutOfMemoryError: Java heap space\nGLFW error 65543: Failed to create window";
        let report = matches_in(text);
        let ids: Vec<&str> = report.iter().map(|item| item.id.as_str()).collect();
        assert!(ids.contains(&"out-of-memory"));
        assert!(ids.contains(&"graphics"));
    }

    #[test]
    fn falls_back_to_generic_crash() {
        let text = "---- Minecraft Crash Report ----\nDescription: Rendering overlay\nsomething odd";
        let matches = matches_in(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "unknown");
    }

    #[test]
    fn reads_newest_crash_report_from_game_dir() {
        let stamp = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("docl-crash-{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("logs")).unwrap();
        fs::create_dir_all(root.join("crash-reports")).unwrap();
        fs::write(root.join("logs").join("latest.log"), "[12:00:00] [main/INFO]: Hello").unwrap();
        fs::write(
            root.join("crash-reports").join("crash-2026-10-22_12.00.00-client.txt"),
            "---- Minecraft Crash Report ----\nDescription: Rendering overlay\njava.lang.OutOfMemoryError: Java heap space",
        )
        .unwrap();

        let mut cfg = Config::default();
        cfg.game_dir = root.to_string_lossy().to_string();
        cfg.version_isolation = false;

        let report = analyze(&cfg, "test-instance").unwrap();
        assert!(report.found);
        assert_eq!(report.source_kind, "crash-report");
        assert!(report.matches.iter().any(|item| item.id == "out-of-memory"));
        assert!(report.headline.contains("Rendering overlay"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extracts_description_and_excerpt() {
        let text = "---- Minecraft Crash Report ----\n// Watchdog\nTime: 2026-10-22\nDescription: Ticking entity\n\njava.lang.NullPointerException";
        assert_eq!(description_of(text).as_deref(), Some("Ticking entity"));
        let excerpt = excerpt_from(text, "crash-report");
        assert!(excerpt.iter().any(|line| line.contains("Description:")));
        assert_eq!(excerpt.len(), 5);
    }
}
