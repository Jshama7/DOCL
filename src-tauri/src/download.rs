use crate::mirror;
use anyhow::{anyhow, bail, Context, Result};
use futures::stream::{FuturesUnordered, StreamExt};
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
    /// 当前阶段的计时基准，进入新阶段会重置
    started: Mutex<Instant>,
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
            started: Mutex::new(Instant::now()),
        })
    }

    /// 进入新的下载阶段：重置本阶段计数与计时基准。
    /// 不能只覆盖 total 而不清 done，否则第二阶段会出现「已下载 885MB / 总量 9MB」，
    /// 百分比直接算成 100%、剩余文件变 0，看起来像卡死。
    pub fn begin_phase(&self, files_total: u64, bytes_total: u64) {
        self.files_done.store(0, Ordering::Relaxed);
        self.bytes_done.store(0, Ordering::Relaxed);
        self.files_total.store(files_total, Ordering::Relaxed);
        self.bytes_total.store(bytes_total, Ordering::Relaxed);
        if let Ok(mut started) = self.started.lock() {
            *started = Instant::now();
        }
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
            elapsed_ms: self
                .started
                .lock()
                .map(|started| started.elapsed().as_millis() as u64)
                .unwrap_or(0),
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
    let mirror_url = mirror::rewrite(url);
    // 镜像未覆盖该主机时 rewrite 原样返回，去重避免对同一地址打两遍
    let raw = if use_mirror {
        vec![mirror_url, url.to_string()]
    } else {
        vec![url.to_string(), mirror_url]
    };
    let mut seen = std::collections::HashSet::new();
    let candidates: Vec<String> = raw.into_iter().filter(|c| seen.insert(c.clone())).collect();
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
            // 首次失败稍等再重试
            if attempt == 0 {
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
        }
        // 还有下一个候选时才等待，避免末尾白等
        if idx + 1 < candidates.len() {
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

/// 等待取消标志被置位，供 select 分支使用
async fn wait_cancel(progress: &Arc<ProgressState>) {
    while !progress.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
}

/// 取消标志存在时才等待，否则永不返回（select 分支保持静默）。
async fn wait_cancel_opt(progress: Option<&Arc<ProgressState>>) {
    match progress {
        Some(progress) => wait_cancel(progress).await,
        None => std::future::pending::<()>().await,
    }
}

/// 竞速用的单次请求：不重试、不走候选列表，避免与竞速层叠加放大请求量。
async fn get_bytes_once(client: &reqwest::Client, url: String) -> Result<Vec<u8>> {
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .with_context(|| format!("请求失败：{url}"))?;
    if !resp.status().is_success() {
        bail!("HTTP {}", resp.status());
    }
    Ok(resp.bytes().await.context("读取响应失败")?.to_vec())
}

/// 竞速请求：先发一路，超过 `spawn_delay_ms` 没回再补下一路（轮流走镜像/官方），
/// 谁先成功用谁。只用于版本清单、资源索引这类关键小请求；大文件竞速会成倍浪费带宽。
/// 补位间隔要按体积给：小 JSON 250ms，几 MB 的索引应放宽，否则正常网络也会补位。
pub async fn get_bytes_race(
    client: &reqwest::Client,
    url: &str,
    use_mirror: bool,
    progress: Option<&Arc<ProgressState>>,
    lanes: usize,
    spawn_delay_ms: u64,
) -> Result<Vec<u8>> {
    let lanes = lanes.clamp(1, 4);
    let mirror_url = mirror::rewrite(url);
    let ordered = if use_mirror {
        vec![mirror_url, url.to_string()]
    } else {
        vec![url.to_string(), mirror_url]
    };
    let mut seen = std::collections::HashSet::new();
    let targets: Vec<String> = ordered.into_iter().filter(|c| seen.insert(c.clone())).collect();

    let mut pending = FuturesUnordered::new();
    let mut started = 0usize;
    let mut last_err: Option<anyhow::Error> = None;

    loop {
        if started < lanes {
            let target = targets[started % targets.len()].clone();
            pending.push(get_bytes_once(client, target));
            started += 1;
        }
        if pending.is_empty() {
            break;
        }
        let can_spawn = started < lanes;
        tokio::select! {
            biased;
            _ = wait_cancel_opt(progress) => bail!("下载已取消"),
            result = pending.next() => {
                match result {
                    Some(Ok(bytes)) => return Ok(bytes),
                    Some(Err(err)) => last_err = Some(err),
                    None => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(spawn_delay_ms)), if can_spawn => {}
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("网络请求失败：{url}")))
}

/// 竞速获取并解析 JSON。
pub async fn get_json_race<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    use_mirror: bool,
    progress: Option<&Arc<ProgressState>>,
) -> Result<T> {
    let bytes = get_bytes_race(client, url, use_mirror, progress, 3, 250).await?;
    serde_json::from_slice(&bytes).context("解析 JSON 失败")
}

/* ---------- 全局限速 ---------- */

/// 全局下载限速（字节/秒），0 表示不限速；由设置页通过 set_speed_limit 更新
static SPEED_LIMIT: AtomicU64 = AtomicU64::new(0);
static WINDOW_BYTES: AtomicU64 = AtomicU64::new(0);
static WINDOW_START: Mutex<Option<Instant>> = Mutex::new(None);

pub fn set_speed_limit(bytes_per_sec: u64) {
    SPEED_LIMIT.store(bytes_per_sec, Ordering::Relaxed);
}

/// 用户是否强制「仅镜像」：此时不看智能排序，全部镜像优先
static FORCE_MIRROR: AtomicBool = AtomicBool::new(false);

pub fn set_force_mirror(value: bool) {
    FORCE_MIRROR.store(value, Ordering::Relaxed);
}

/// 滑动 1 秒窗口的简单限速：所有并发任务共享同一个配额。
/// 超出配额时挂起等待到下一窗口，保证平均速率不超过限制。
async fn throttle(bytes: u64) {
    let limit = SPEED_LIMIT.load(Ordering::Relaxed);
    // 未设限，或单块就超过整秒配额时直接放行，否则永远等不到够用的窗口
    if limit == 0 || bytes > limit {
        return;
    }
    loop {
        let now = Instant::now();
        let window_start = {
            let mut guard = WINDOW_START.lock().unwrap();
            match *guard {
                Some(start) if now.duration_since(start).as_millis() < 1000 => start,
                _ => {
                    *guard = Some(now);
                    WINDOW_BYTES.store(0, Ordering::Relaxed);
                    now
                }
            }
        };
        let elapsed = now.duration_since(window_start).as_millis() as u64;
        let used = WINDOW_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
        if used <= limit {
            return;
        }
        // 配额不足：退回记账，睡到本窗口结束再试。
        // 窗口可能已被其他任务重置，用饱和减法避免 u64 下溢成巨大值
        let _ = WINDOW_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            Some(value.saturating_sub(bytes))
        });
        let remain_ms = 1000u64.saturating_sub(elapsed).max(10);
        tokio::time::sleep(Duration::from_millis(remain_ms)).await;
    }
}

/// 镜像速度下限（字节/秒）：低于它在探测窗口后判定源不合格并换源，PCL 同样用 256 KB/s
const SLOW_SOURCE_BYTES_PER_SEC: u64 = 256 * 1024;
/// 速度探测窗口，太短容易被网络抖动误判
const SLOW_PROBE_WINDOW: Duration = Duration::from_secs(6);

async fn fetch_to_file(
    client: &reqwest::Client,
    url: &str,
    task: &FileTask,
    progress: &Arc<ProgressState>,
    allow_slow_bail: bool,
) -> Result<()> {
    if let Some(parent) = task.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = task.dest.with_extension("part");
    let _ = tokio::fs::remove_file(&tmp).await;

    // 连接阶段必须有超时：CDN 握手挂起时 send() 会一直等，取消也无法打断
    let resp = tokio::time::timeout(Duration::from_secs(30), client.get(url).send())
        .await
        .map_err(|_| anyhow!("连接超时：{}", task.label))?
        .with_context(|| format!("连接失败：{}", task.label))?;
    if !resp.status().is_success() {
        bail!("服务器返回 {}", resp.status());
    }

    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut hasher = Sha1::new();
    let mut stream = resp.bytes_stream();
    // 连接已完成，从这里开始算有效速度
    let probe_start = Instant::now();
    let mut probe_bytes = 0u64;
    let mut probed = false;
    loop {
        if progress.is_cancelled() {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp).await;
            bail!("下载已取消");
        }
        // 单块读取加超时，避免数据流卡住时永远等不到下一个 chunk
        let next = tokio::time::timeout(Duration::from_secs(30), stream.next())
            .await
            .map_err(|_| anyhow!("下载超时（30 秒无数据）：{}", task.label))?;
        let Some(chunk) = next else { break };
        let chunk = chunk.context("读取下载数据失败")?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        progress
            .bytes_done
            .fetch_add(chunk.len() as u64, Ordering::Relaxed);
        throttle(chunk.len() as u64).await;

        // 下载源慢到不可用时提前放弃，交给上层换源（用户主动限速时不判定，否则必然误切）
        if allow_slow_bail && !probed {
            probe_bytes += chunk.len() as u64;
            let elapsed = probe_start.elapsed();
            if elapsed >= SLOW_PROBE_WINDOW {
                probed = true;
                let speed = probe_bytes as f64 / elapsed.as_secs_f64();
                if SPEED_LIMIT.load(Ordering::Relaxed) == 0
                    && speed < SLOW_SOURCE_BYTES_PER_SEC as f64
                {
                    drop(file);
                    let _ = tokio::fs::remove_file(&tmp).await;
                    // 这部分数据已经丢弃，进度要扣回避免虚高
                    progress
                        .bytes_done
                        .fetch_sub(probe_bytes, Ordering::Relaxed);
                    bail!(
                        "下载源速度过慢（{} KB/s）：{}，换源重试",
                        (speed / 1024.0) as u64,
                        task.label
                    );
                }
            }
        }
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
    // 智能排序：Modrinth CDN 国内多数网络不可达，镜像优先；
    // CurseForge 等官方源通常更快，官方优先，慢了再由慢速降级切走。
    // 用户显式选「仅镜像」时不排序，一律镜像优先。
    let mirror_first = mirror_healthy
        && (FORCE_MIRROR.load(Ordering::Relaxed) || mirror::prefer_mirror_first(&task.url));
    let order = if mirror_first {
        [mirror_url, official_url]
    } else {
        [official_url, mirror_url]
    };

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..3usize {
        if progress.is_cancelled() {
            bail!("下载已取消");
        }
        let uses_mirror = if mirror_first { attempt % 2 == 0 } else { attempt % 2 == 1 };
        let url = &order[attempt % 2];
        // 前两次尝试都允许因过慢切源，最后一次只求下完，避免两个源都慢时直接失败
        let allow_slow_bail = attempt < 2;
        match fetch_to_file(client, url, task, progress, allow_slow_bail).await {
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

/// 下载任务并发上限。设置页滑条、各调用点都必须引用它，
/// 避免像整合包那样各自硬编码成 32 导致上限不一致。
pub const MAX_CONCURRENCY: usize = 64;

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
    progress.begin_phase(total_files, total_bytes);
    progress.active.store(true, Ordering::Relaxed);

    if tasks.is_empty() {
        progress.active.store(false, Ordering::Relaxed);
        emit_now(app, &progress);
        return Ok(());
    }

    start_ticker(app.clone(), progress.clone());
    emit_now(app, &progress);

    let conc = concurrency.clamp(1, MAX_CONCURRENCY);
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
    // 取消不能只靠任务自己返回错误：所有并发请求都卡在网络上时，
    // 主循环永远等不到结果。这里用 select 监听取消标志，一旦取消就
    // 跳出并丢弃 stream，正在进行的请求随之被打断。
    loop {
        tokio::select! {
            biased;
            _ = wait_cancel(&progress) => {
                first_err = Some(anyhow!("下载已取消"));
                break;
            }
            next = stream.next() => {
                match next {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        if progress.is_cancelled() {
                            first_err = Some(err);
                            break;
                        }
                        if first_err.is_none() {
                            first_err = Some(err);
                        }
                    }
                    None => break,
                }
            }
        }
    }
    drop(stream);

    progress.active.store(false, Ordering::Relaxed);
    emit_now(app, &progress);

    if let Some(err) = first_err {
        return Err(err);
    }
    Ok(())
}
