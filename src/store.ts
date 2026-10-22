import { computed, reactive } from "vue";
import {
  api,
  onGameExit,
  onGameLog,
  onInstallProgress,
  onMsLogin,
  onMsLoginAccount,
  type AccountView,
  type Config,
  type DeviceCode,
  type InstalledVersion,
  type JavaInfo,
  type LogLine,
  type ProgressSnapshot,
  type VersionSummary,
} from "./api";

export type Page = "home" | "versions" | "download" | "accounts" | "settings";

export const store = reactive({
  page: "home" as Page,
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

export async function refreshAccounts() {
  store.accounts = await api.getAccounts();
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

export async function installVersion(id: string) {
  if (store.installingId) {
    notify("已有安装任务正在进行", "err");
    return;
  }
  store.installingId = id;
  store.progress = null;
  try {
    await api.installVersion(id);
    await refreshInstalled();
    await refreshConfig();
    notify(`${id} 安装完成`);
  } catch (err) {
    notify(`安装失败：${String(err)}`, "err");
  } finally {
    store.installingId = null;
    window.setTimeout(() => {
      if (!store.progress?.active) store.progress = null;
    }, 800);
  }
}

export async function installLoader(versionId: string, loader: string) {
  if (store.installingId) {
    notify("已有安装任务正在进行", "err");
    return;
  }
  store.installingId = `${versionId} · ${loader}`;
  store.progress = null;
  try {
    await api.installLoader(versionId, loader);
    await refreshInstalled();
    await refreshConfig();
    notify(`${loader} 安装完成`);
  } catch (err) {
    notify(`安装失败：${String(err)}`, "err");
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

  await Promise.allSettled([refreshInstalled(), refreshAccounts(), refreshJava()]);

  await onInstallProgress((p) => {
    store.progress = p;
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
