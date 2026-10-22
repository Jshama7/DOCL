import { computed, reactive } from "vue";
import {
  api,
  onGameExit,
  onGameLog,
  onInstallProgress,
  onMsLogin,
  onMsLoginAccount,
  type AccountView,
  type AccountSkin,
  type Config,
  type DeviceCode,
  type InstalledVersion,
  type InstanceMeta,
  type JavaInfo,
  type LogLine,
  type ProgressSnapshot,
  type VersionSummary,
} from "./api";

export type Page = "home" | "versions" | "download" | "accounts" | "instance" | "settings";

export const store = reactive({
  page: "home" as Page,
  instanceId: null as string | null,
  metas: {} as Record<string, InstanceMeta>,
  communityContext: null as {
    kind: string;
    instanceId: string;
    gameVersion: string;
    loader: string;
  } | null,
  config: null as Config | null,
  versions: [] as VersionSummary[],
  installed: [] as InstalledVersion[],
  accounts: [] as AccountView[],
  javaList: [] as JavaInfo[],
  loadingVersions: false,
  progress: null as ProgressSnapshot | null,
  installingId: null as string | null,
  log: [] as LogLine[],
  logOpen: false,
  gameRunning: false,
  backgroundUrl: "",
  backgroundPreset: "",
  downloadPanel: "hidden" as "hidden" | "open" | "ball",
  lastProgress: null as ProgressSnapshot | null,
  lastJobTitle: "",
  lastJobError: null as string | null,
  accountSkin: null as AccountSkin | null,
  skinLoading: false,
  skinEditorOpen: false,
  toast: null as { text: string; kind: "ok" | "err" } | null,
  msLogin: null as {
    code: DeviceCode | null;
    status: "loading" | "waiting" | "ok" | "err";
    message?: string;
  } | null,
});

export const selectedVersion = computed<InstalledVersion | null>(() => {
  const id = store.config?.selected_version;
  return store.installed.find((v) => v.id === id) ?? store.installed[0] ?? null;
});

export const selectedAccount = computed<AccountView | null>(() => {
  const id = store.config?.selected_account;
  return store.accounts.find((a) => a.id === id) ?? store.accounts[0] ?? null;
});

let toastTimer: number | undefined;

export function notify(text: string, kind: "ok" | "err" = "ok") {
  store.toast = { text, kind };
  if (toastTimer) window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    store.toast = null;
  }, 3200);
}

export async function refreshConfig() {
  store.config = await api.getConfig();
}

export async function refreshInstalled() {
  store.installed = await api.installedVersions();
}

export async function refreshInstanceMetas() {
  try {
    store.metas = await api.instanceMetas();
  } catch {
    store.metas = {};
  }
}

export function openInstance(id: string) {
  store.instanceId = id;
  store.page = "instance";
}

/// 从版本设置跳到下载页，并带上当前实例的版本与加载器。
export function openCommunity(
  kind: string,
  instanceId: string,
  gameVersion: string,
  loader: string
) {
  store.communityContext = { kind, instanceId, gameVersion, loader };
  store.page = "download";
}

export async function refreshAccounts() {
  store.accounts = await api.getAccounts();
  await loadSelectedSkin();
}

export async function loadSelectedSkin() {
  const account = selectedAccount.value;
  if (!account) {
    store.accountSkin = null;
    return;
  }
  store.skinLoading = true;
  try {
    store.accountSkin = await api.getAccountSkin(account.id);
  } catch {
    store.accountSkin = null;
  } finally {
    store.skinLoading = false;
  }
}

export async function refreshJava() {
  store.javaList = await api.detectJava();
}

export async function refreshVersions(force = false) {
  if (store.loadingVersions) return;
  store.loadingVersions = true;
  try {
    store.versions = await api.fetchVersions(force);
  } catch (err) {
    notify(`获取版本列表失败：${String(err)}`, "err");
  } finally {
    store.loadingVersions = false;
  }
}

export async function saveConfig(patch: Partial<Config>) {
  if (!store.config) return;
  const next = { ...store.config, ...patch };
  try {
    store.config = await api.saveConfig(next);
    applyTheme(store.config.theme);
    await refreshInstalled();
    await refreshJava();
  } catch (err) {
    notify(`保存设置失败：${String(err)}`, "err");
  }
}

export function applyTheme(theme?: string | null) {
  document.documentElement.dataset.theme = theme || "paper";
}

export async function loadBackground() {
  const path = store.config?.background_image;
  if (!path) {
    store.backgroundUrl = "";
    store.backgroundPreset = "";
    return;
  }
  if (path.startsWith("builtin:")) {
    store.backgroundPreset = path.slice("builtin:".length);
    store.backgroundUrl = "";
    return;
  }
  store.backgroundPreset = "";
  try {
    store.backgroundUrl = await api.backgroundDataUrl(path);
  } catch {
    store.backgroundUrl = "";
  }
}

export async function useBuiltinBackground(name: string) {
  store.backgroundPreset = name;
  store.backgroundUrl = "";
  await saveConfig({ background_image: `builtin:${name}` });
}

export async function setBackground(path: string) {
  try {
    const info = await api.importBackground(path);
    store.backgroundUrl = info.data_url;
    store.backgroundPreset = "";
    await saveConfig({ background_image: info.path });
    notify("背景图片已应用");
  } catch (err) {
    notify(`设置背景失败：${String(err)}`, "err");
  }
}

export async function clearBackground() {
  store.backgroundUrl = "";
  store.backgroundPreset = "";
  await saveConfig({ background_image: null });
}

function primeProgress(stage: string, message: string) {
  store.progress = {
    stage,
    message,
    files_done: 0,
    files_total: 0,
    bytes_done: 0,
    bytes_total: 0,
    elapsed_ms: 0,
    active: true,
  };
  store.lastProgress = store.progress;
}

export async function installVersion(id: string) {
  if (store.installingId) {
    notify("已有安装任务正在进行", "err");
    return;
  }
  store.installingId = id;
  store.progress = null;
  store.lastProgress = null;
  store.lastJobError = null;
  store.lastJobTitle = id;
  store.downloadPanel = "open";
  primeProgress("install", `准备安装 ${id}…`);
  try {
    await api.installVersion(id);
    await refreshInstalled();
    await refreshConfig();
    notify(`${id} 安装完成`);
  } catch (err) {
    store.lastJobError = String(err);
    notify(`安装失败：${String(err)}`, "err");
  } finally {
    store.installingId = null;
    window.setTimeout(() => {
      if (!store.progress?.active) store.progress = null;
    }, 800);
  }
}

export async function installLoader(
  versionId: string,
  loader: string,
  loaderVersion?: string
) {
  if (store.installingId) {
    notify("已有安装任务正在进行", "err");
    return;
  }
  store.installingId = `${versionId} · ${loader}`;
  store.progress = null;
  store.lastProgress = null;
  store.lastJobError = null;
  store.lastJobTitle = `${versionId} · ${loader}${loaderVersion ? ` ${loaderVersion}` : ""}`;
  store.downloadPanel = "open";
  primeProgress("install", `准备安装 ${loader}…`);
  try {
    await api.installLoader(versionId, loader, loaderVersion);
    await refreshInstalled();
    await refreshConfig();
    notify(`${loader} 安装完成`);
  } catch (err) {
    store.lastJobError = String(err);
    notify(`安装失败：${String(err)}`, "err");
  } finally {
    store.installingId = null;
    window.setTimeout(() => {
      if (!store.progress?.active) store.progress = null;
    }, 800);
  }
}

export async function installCfFile(
  versionId: string,
  kind: string,
  url: string,
  fileName: string,
  title: string
) {
  if (store.installingId) {
    notify("已有下载任务正在进行", "err");
    return;
  }
  store.installingId = `${versionId} · ${fileName}`;
  store.progress = null;
  store.lastProgress = null;
  store.lastJobError = null;
  store.lastJobTitle = title;
  store.downloadPanel = "open";
  primeProgress("install", `准备下载 ${title}…`);
  try {
    await api.cfInstallFile(versionId, kind, url, fileName);
    notify(`${title} 下载完成`);
  } catch (err) {
    store.lastJobError = String(err);
    notify(`下载失败：${String(err)}`, "err");
  } finally {
    store.installingId = null;
    window.setTimeout(() => {
      if (!store.progress?.active) store.progress = null;
    }, 800);
  }
}

export async function importModpack(
  path: string,
  options: import("./api").ModpackImportOptions,
  title: string
) {
  if (store.installingId) {
    notify("已有下载任务正在进行", "err");
    return null;
  }
  store.installingId = `整合包 · ${title}`;
  store.progress = null;
  store.lastProgress = null;
  store.lastJobError = null;
  store.lastJobTitle = `导入整合包 · ${title}`;
  store.downloadPanel = "open";
  primeProgress("install", `准备导入 ${title}…`);
  try {
    const result = await api.modpackImport(path, options);
    await refreshInstalled();
    await refreshConfig();
    await refreshInstanceMetas();
    notify(
      result.failed.length
        ? `已导入 ${result.display_name}，${result.failed.length} 个文件未能下载`
        : `已导入 ${result.display_name}`
    );
    if (result.failed.length) {
      store.lastJobError = result.failed.slice(0, 5).join("；");
    }
    return result;
  } catch (err) {
    store.lastJobError = String(err);
    notify(`导入失败：${String(err)}`, "err");
    return null;
  } finally {
    store.installingId = null;
    window.setTimeout(() => {
      if (!store.progress?.active) store.progress = null;
    }, 800);
  }
}

export async function exportModpack(
  versionId: string,
  options: import("./api").ExportOptions,
  title: string
) {
  if (store.installingId) {
    notify("已有任务正在进行", "err");
    return null;
  }
  store.installingId = `导出 · ${title}`;
  store.progress = null;
  store.lastProgress = null;
  store.lastJobError = null;
  store.lastJobTitle = `导出整合包 · ${title}`;
  store.downloadPanel = "open";
  primeProgress("install", "准备导出整合包…");
  try {
    const result = await api.modpackExport(versionId, options);
    notify(`已导出到 ${result.paths[0] ?? options.target}`);
    return result;
  } catch (err) {
    store.lastJobError = String(err);
    notify(`导出失败：${String(err)}`, "err");
    return null;
  } finally {
    store.installingId = null;
    window.setTimeout(() => {
      if (!store.progress?.active) store.progress = null;
    }, 800);
  }
}

export async function installJava(major: number) {
  if (store.installingId) {
    notify("已有安装任务正在进行", "err");
    return;
  }
  store.installingId = `Java ${major}`;
  store.progress = null;
  store.lastProgress = null;
  store.lastJobError = null;
  store.lastJobTitle = `Java ${major}`;
  store.downloadPanel = "open";
  primeProgress("java", `准备安装 Java ${major}…`);
  try {
    const info = await api.installJava(major);
    await refreshJava();
    await refreshConfig();
    notify(`Java ${major} 已安装`);
    return info;
  } catch (err) {
    store.lastJobError = String(err);
    notify(`安装 Java 失败：${String(err)}`, "err");
    return null;
  } finally {
    store.installingId = null;
    window.setTimeout(() => {
      if (!store.progress?.active) store.progress = null;
    }, 800);
  }
}

export async function cancelInstall() {
  await api.cancelInstall();
  notify("已请求取消安装");
}

export async function selectVersion(id: string) {
  await saveConfig({ selected_version: id });
}

export async function selectAccount(id: string) {
  await api.selectAccount(id);
  await refreshConfig();
  await loadSelectedSkin();
}

export async function createOffline(name: string) {
  try {
    await api.createOfflineAccount(name);
    await refreshAccounts();
    await refreshConfig();
    notify(`已添加离线账号 ${name}`);
  } catch (err) {
    notify(String(err), "err");
  }
}

export async function removeAccount(id: string) {
  try {
    await api.removeAccount(id);
    await refreshAccounts();
    await refreshConfig();
    notify("账号已移除");
  } catch (err) {
    notify(String(err), "err");
  }
}

export async function beginMsLogin() {
  store.msLogin = { code: null, status: "loading" };
  try {
    const code = await api.msLoginBegin();
    store.msLogin = { code, status: "waiting" };
  } catch (err) {
    store.msLogin = { code: null, status: "err", message: String(err) };
  }
}

export async function launchVersion(id?: string) {
  const target = id ?? selectedVersion.value?.id;
  if (!target) {
    notify("请先安装一个游戏版本", "err");
    store.page = "download";
    return;
  }
  if (store.gameRunning) {
    try {
      await api.killGame();
    } catch (err) {
      notify(String(err), "err");
    }
    return;
  }
  store.gameRunning = true;
  try {
    await api.launchVersion(target);
    notify(`正在启动 ${target}`);
  } catch (err) {
    store.gameRunning = false;
    const message = String(err);
    if (message.includes("LAUNCH_NO_JAVA")) {
      notify(message.replace("LAUNCH_NO_JAVA:", ""), "err");
      store.page = "settings";
    } else {
      notify(`启动失败：${message}`, "err");
    }
  }
}

export async function uninstallVersion(id: string) {
  try {
    await api.deleteVersion(id);
    await refreshInstalled();
    if (store.config?.selected_version === id) {
      await saveConfig({ selected_version: store.installed[0]?.id ?? null });
    }
    await refreshInstanceMetas();
    notify(`已删除 ${id}`);
  } catch (err) {
    notify(`删除失败：${String(err)}`, "err");
  }
}

let initialized = false;

export async function initApp() {
  if (initialized) return;
  initialized = true;

  try {
    store.config = await api.getConfig();
    applyTheme(store.config.theme);
    await loadBackground();
  } catch (err) {
    notify(`读取配置失败：${String(err)}`, "err");
  }

  await Promise.allSettled([
    refreshInstalled(),
    refreshAccounts(),
    refreshJava(),
    refreshInstanceMetas(),
  ]);

  await onInstallProgress((p) => {
    store.progress = p;
    store.lastProgress = p;
  });

  await onGameLog((line) => {
    store.log.push(line);
    if (store.log.length > 3000) store.log.splice(0, store.log.length - 3000);
  });

  await onGameExit((exit) => {
    store.gameRunning = false;
    notify(
      exit.code === 0 ? "游戏已退出" : `游戏异常退出（代码 ${exit.code ?? "未知"}）`,
      exit.code === 0 ? "ok" : "err"
    );
    store.logOpen = true;
  });

  await onMsLoginAccount(async (account) => {
    if (!store.accounts.some((a) => a.id === account.id)) {
      store.accounts.push(account);
    }
    await refreshAccounts();
    await api.selectAccount(account.id);
    await refreshConfig();
    if (store.msLogin) store.msLogin.status = "ok";
    notify(`已登录：${account.name}`);
  });

  await onMsLogin((payload) => {
    if (store.msLogin) {
      store.msLogin.status = "err";
      store.msLogin.message = payload.message ?? "登录失败";
    }
  });
}
