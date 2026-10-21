import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";

export interface Config {
  game_dir: string;
  java_path: string | null;
  memory_mb: number;
  java_args: string;
  window_width: number;
  window_height: number;
  fullscreen: boolean;
  download_source: string;
  concurrency: number;
  animations: boolean;
  version_isolation: boolean;
  selected_version: string | null;
  selected_account: string | null;
}

export interface VersionSummary {
  id: string;
  kind: string;
  release_time: string | null;
  installed: boolean;
}

export interface InstalledVersion {
  id: string;
  kind: string;
  jar_size: number;
  has_jar: boolean;
  last_modified: number;
  java_major: number;
}

export interface JavaInfo {
  path: string;
  version: string;
  major: number;
  source: string;
}

export interface AccountView {
  id: string;
  kind: string;
  name: string;
  uuid: string;
}

export interface DeviceCode {
  user_code: string;
  verification_uri: string;
  message: string;
  expires_in: number;
  interval: number;
}

export interface ProgressSnapshot {
  stage: string;
  message: string;
  files_done: number;
  files_total: number;
  bytes_done: number;
  bytes_total: number;
  elapsed_ms: number;
  active: boolean;
}

export interface LogLine {
  stream: string;
  line: string;
}

export interface GameExit {
  code: number | null;
  pid: number;
}

export const api = {
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<Config>("save_config", { config }),
  fetchVersions: (force = false) => invoke<VersionSummary[]>("fetch_versions", { force }),
  installedVersions: () => invoke<InstalledVersion[]>("installed_versions"),
  installVersion: (versionId: string) => invoke<InstalledVersion>("install_version", { versionId }),
  cancelInstall: () => invoke<void>("cancel_install"),
  deleteVersion: (versionId: string) => invoke<void>("delete_version", { versionId }),
  getAccounts: () => invoke<AccountView[]>("get_accounts"),
  createOfflineAccount: (name: string) => invoke<AccountView>("create_offline_account", { name }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  selectAccount: (id: string) => invoke<void>("select_account", { id }),
  msLoginBegin: () => invoke<DeviceCode>("ms_login_begin"),
  detectJava: () => invoke<JavaInfo[]>("detect_java"),
  installJava: (major: number) => invoke<JavaInfo>("install_java", { major }),
  launchVersion: (versionId: string) => invoke<number>("launch_version", { versionId }),
  killGame: () => invoke<void>("kill_game"),
};

export function onInstallProgress(cb: (p: ProgressSnapshot) => void): Promise<UnlistenFn> {
  return listen<ProgressSnapshot>("install-progress", (e) => cb(e.payload));
}

export function onGameLog(cb: (l: LogLine) => void): Promise<UnlistenFn> {
  return listen<LogLine>("game-log", (e) => cb(e.payload));
}

export function onGameExit(cb: (e: GameExit) => void): Promise<UnlistenFn> {
  return listen<GameExit>("game-exit", (e) => cb(e.payload));
}

export function onMsLoginAccount(cb: (a: AccountView) => void): Promise<UnlistenFn> {
  return listen<AccountView>("ms-login-account", (e) => cb(e.payload));
}

export function onMsLogin(
  cb: (p: { ok: boolean; message?: string }) => void
): Promise<UnlistenFn> {
  return listen<{ ok: boolean; message?: string }>("ms-login", (e) => cb(e.payload));
}

export async function pickDirectory(): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, title: "选择游戏目录" });
  return typeof result === "string" ? result : null;
}

export function openFolder(path: string) {
  return openPath(path);
}

export function openExternal(url: string) {
  return openUrl(url);
}
