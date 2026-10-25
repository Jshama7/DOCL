use crate::mirror;
use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct FileTask {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: u64,
    pub label: String,
}

#[derive(Debug)]
pub struct ProgressState {
    pub stage: String,
    message: Mutex<String>,
    /// 依次经过的安装阶段，供下载面板显示成步骤列表
    steps: Mutex<Vec<ProgressStep>>,
    pub files_done: AtomicU64,
    pub files_total: AtomicU64,
    pub bytes_done: AtomicU64,
    pub bytes_total: AtomicU64,
    pub cancel: AtomicBool,
    pub active: AtomicBool,
    /// 是否已有刷新联动的任务，避免重复起多个
    ticking: AtomicBool,
    started: Instant,
}

/// 面板最多展示的步骤数量，避免加载器日志刷屏
const MAX_STEPS: usize = 8;

#[derive(Clone, Debug, Serialize)]
pub struct ProgressStep {
    pub label: String,
    pub done: bool,
}

#[derive(Clone, Serialize)]
pub struct ProgressSnapshot {
    pub stage: String,
    pub message: String,
    pub steps: Vec<ProgressStep>,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub elapsed_ms: u64,
    pub active: bool,
}

impl ProgressState {
    pub fn new(stage: &str) -> Arc<Self> {
        Arc::new(Self {
            stage: stage.to_string(),
            message: Mutex::new(String::new()),
            steps: Mutex::new(Vec::new()),
            files_done: AtomicU64::new(0),
            files_total: AtomicU64::new(0),
            bytes_done: AtomicU64::new(0),
            bytes_total: AtomicU64::new(0),
            cancel: AtomicBool::new(false),
            active: AtomicBool::new(true),
            ticking: AtomicBool::new(false),
            started: Instant::now(),
        })
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        let msg = msg.into();
        if msg.trim().is_empty() {
            return;
        }
        if let Ok(mut steps) = self.steps.lock() {
            let repeated = steps
                .last()
                .map(|step| step.label == msg)
                .unwrap_or(false);
            if !repeated {
                if let Some(last) = steps.last_mut() {
                    last.done = true;
                }
                steps.push(ProgressStep {
                    label: msg.clone(),
                    done: false,
                });
                if steps.len() > MAX_STEPS {
                    steps.remove(0);
                }
            }
        }
        if let Ok(mut guard) = self.message.lock() {
            *guard = msg;
        }
    }

    /// 只更新当前阶段的细节（安装器实时输出），不新增步骤。
    pub fn set_detail(&self, msg: impl Into<String>) {
        let msg = msg.into();
        if msg.trim().is_empty() {
            return;
        }
        if let Ok(mut steps) = self.steps.lock() {
            if let Some(last) = steps.last_mut() {
                if !last.done {
                    last.label = msg.clone();
                }
            }
        }
        if let Ok(mut guard) = self.message.lock() {
            *guard = msg;
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> ProgressSnapshot {
        let active = self.active.load(Ordering::Relaxed);
        let mut steps = self
            .steps
            .lock()
            .map(|list| list.clone())
            .unwrap_or_default();
        // 任务结束后最后一步也视为完成
        if !active {
            if let Some(last) = steps.last_mut() {
                last.done = true;
            }
        }
        ProgressSnapshot {
            stage: self.stage.clone(),
            message: self
                .message
                .lock()
                .map(|m| m.clone())
                .unwrap_or_default(),
            steps,
            files_done: self.files_done.load(Ordering::Relaxed),
            files_total: self.files_total.load(Ordering::Relaxed),
            bytes_done: self.bytes_done.load(Ordering::Relaxed),
            bytes_total: self.bytes_total.load(Ordering::Relaxed),
            elapsed_ms: self.started.elapsed().as_millis() as u64,
            active,
        }
    }
}

pub fn emit_now(app: &AppHandle, progress: &Arc<ProgressState>) {
    let _ = app.emit("install-progress", progress.snapshot());
}

fn start_ticker(app: AppHandle, progress: Arc<ProgressState>) {
    // 已经在刷新就不重复起任务；任务结束后由本轮循环复位
    if progress.ticking.swap(true, Ordering::Relaxed) {
        return;
    }
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(180)).await;
            let _ = app.emit("install-progress", progress.snapshot());
            if !progress.active.load(Ordering::Relaxed) {
                break;
            }
        }
        progress.ticking.store(false, Ordering::Relaxed);
    });
}

/// 进入一个没有逐文件进度的阶段：更新文案、保持界面刷新。
pub fn begin_stage(app: &AppHandle, progress: &Arc<ProgressState>, message: impl Into<String>) {
    progress.set_message(message);
    progress.active.store(true, Ordering::Relaxed);
    start_ticker(app.clone(), progress.clone());
    emit_now(app, progress);
}

pub async fn get_bytes(
    client: &reqwest::Client,
    url: &str,
    use_mirror: bool,
) -> Result<Vec<u8>> {
    let mut last_err: Option<anyhow::Error> = None;
    let candidates = if use_mirror {
        vec![mirror::rewrite(url), url.to_string()]
    } else {
        vec![url.to_string(), mirror::rewrite(url)]
    };
    // 单文件请求必须有总超时：connect_timeout 只管握手，
    // 响应体挂起（官方源被墙断流）会无限等待，表现为「卡在某一步」
    for (idx, candidate) in candidates.iter().enumerate() {
        for attempt in 0..2 {
            let request = client
                .get(candidate)
                .timeout(Duration::from_secs(45));
            match request.send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.bytes().await {
                        Ok(bytes) => return Ok(bytes.to_vec()),
                        Err(err) => last_err = Some(err.into()),
                    }
                }
                Ok(resp) => last_err = Some(anyhow!("HTTP {}", resp.status())),
                Err(err) => last_err = Some(err.into()),
            }
            // 首次失败稍等再重试；两个候选共 4 次机会
            if attempt == 0 {
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
        }
        if idx == 0 {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("网络请求失败：{url}")))
}

pub async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    use_mirror: bool,
) -> Result<T> {
    let bytes = get_bytes(client, url, use_mirror).await?;
    serde_json::from_slice(&bytes).context("解析 JSON 失败")
}

async fn fetch_to_file(
    client: &reqwest::Client,
    url: &str,
    task: &FileTask,
    progress: &Arc<ProgressState>,
) -> Result<()> {
    if let Some(parent) = task.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = task.dest.with_extension("part");
    let _ = tokio::fs::remove_file(&tmp).await;

    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("连接失败：{}", task.label))?;
    if !resp.status().is_success() {
        bail!("服务器返回 {}", resp.status());
    }

    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut hasher = Sha1::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("读取下载数据失败")?;
        if progress.is_cancelled() {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp).await;
            bail!("下载已取消");
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        progress
            .bytes_done
            .fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }
    file.flush().await?;
    drop(file);

    let actual = format!("{:x}", hasher.finalize());
    if let Some(expected) = &task.sha1 {
        if !expected.is_empty() && !expected.eq_ignore_ascii_case(&actual) {
            let _ = tokio::fs::remove_file(&tmp).await;
            bail!("文件校验失败（SHA1 不匹配）：{}", task.label);
        }
    }
    if task.dest.exists() {
        let _ = tokio::fs::remove_file(&task.dest).await;
    }
    tokio::fs::rename(&tmp, &task.dest).await?;
    Ok(())
}

async fn download_one(
    client: &reqwest::Client,
    task: &FileTask,
    use_mirror: bool,
    mirror_failures: &AtomicU64,
    progress: &Arc<ProgressState>,
) -> Result<()> {
    if progress.is_cancelled() {
        bail!("下载已取消");
    }
    if task.dest.exists() {
        if let Ok(meta) = tokio::fs::metadata(&task.dest).await {
            if task.size == 0 || meta.len() == task.size {
                progress.files_done.fetch_add(1, Ordering::Relaxed);
                progress
                    .bytes_done
                    .fetch_add(meta.len(), Ordering::Relaxed);
                return Ok(());
            }
        }
    }

    let mirror_url = mirror::rewrite(&task.url);
    let official_url = task.url.clone();
    let mirror_healthy = use_mirror && mirror_failures.load(Ordering::Relaxed) < 4;
    let order = if mirror_healthy {
        [mirror_url, official_url]
    } else {
        [official_url, mirror_url]
    };

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..3usize {
        if progress.is_cancelled() {
            bail!("下载已取消");
        }
        let uses_mirror = if mirror_healthy { attempt % 2 == 0 } else { attempt % 2 == 1 };
        let url = &order[attempt % 2];
        match fetch_to_file(client, url, task, progress).await {
            Ok(()) => {
                progress.files_done.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            Err(err) => {
                if use_mirror && uses_mirror {
                    mirror_failures.fetch_add(1, Ordering::Relaxed);
                }
                last_err = Some(err);
                tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1))).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("下载失败：{}", task.label)))
}

pub async fn run_tasks(
    app: &AppHandle,
    client: &reqwest::Client,
    tasks: Vec<FileTask>,
    concurrency: usize,
    use_mirror: bool,
    progress: Arc<ProgressState>,
) -> Result<()> {
    let total_files = tasks.len() as u64;
    let total_bytes: u64 = tasks.iter().map(|t| t.size).sum();
    progress.files_total.store(total_files, Ordering::Relaxed);
    progress.bytes_total.store(total_bytes, Ordering::Relaxed);
    progress.active.store(true, Ordering::Relaxed);

    if tasks.is_empty() {
        progress.active.store(false, Ordering::Relaxed);
        emit_now(app, &progress);
        return Ok(());
    }

    start_ticker(app.clone(), progress.clone());
    emit_now(app, &progress);

    let conc = concurrency.clamp(1, 32);
    let mirror_failures = Arc::new(AtomicU64::new(0));
    let mut stream = futures::stream::iter(tasks.into_iter().map(|task| {
        let client = client.clone();
        let progress = progress.clone();
        let mirror_failures = mirror_failures.clone();
        async move {
            download_one(&client, &task, use_mirror, &mirror_failures, &progress).await
        }
    }))
    .buffer_unordered(conc);

    let mut first_err: Option<anyhow::Error> = None;
    while let Some(result) = stream.next().await {
        if let Err(err) = result {
            if progress.is_cancelled() {
                first_err = Some(err);
                break;
            }
            if first_err.is_none() {
                first_err = Some(err);
            }
        }
    }

    progress.active.store(false, Ordering::Relaxed);
    emit_now(app, &progress);

    if let Some(err) = first_err {
        return Err(err);
    }
    Ok(())
}
