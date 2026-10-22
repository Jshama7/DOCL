import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";

export interface GameDirEntry {
  name: string;
  path: string;
}

export interface GameDirsView {
  dirs: GameDirEntry[];
  active: string;
  official: string;
}

export interface Config {
  game_dir: string;
  game_dirs: GameDirEntry[];
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
  theme: string;
  background_image: string | null;
  background_blur: number;
  glass_opacity: number;
  glass_blur: number;
}

export interface BackgroundInfo {
  path: string;
  data_url: string;
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

export interface LoaderOption {
  loader: string;
  label: string;
  version: string | null;
  installed: boolean;
  installed_id: string | null;
  supported: boolean;
  note: string;
}

export interface LoaderVersion {
  version: string;
  recommended: boolean;
}

export interface JavaInfo {
  path: string;
  version: string;
  major: number;
  source: string;
}

export interface InstanceContext {
  game_version: string;
  loader: string;
}

export interface InstanceMeta {
  display_name: string | null;
  description: string;
  icon: string;
  category: string;
  favorite: boolean;
  memory_mb: number | null;
  java_path: string | null;
  java_args: string | null;
  window_width: number | null;
  window_height: number | null;
  fullscreen: boolean | null;
}

export interface InstanceCounts {
  mods: number;
  resourcepacks: number;
  shaderpacks: number;
  saves: number;
  screenshots: number;
}

export interface InstanceInfo extends InstanceMeta {
  id: string;
  game_version: string;
  loader: string;
  kind: string;
  java_major: number;
  path: string;
  installed_at: number;
  counts: InstanceCounts;
}

export interface InstanceFile {
  name: string;
  display_name: string;
  size: number;
  modified: number;
  enabled: boolean;
  is_dir: boolean;
}

export interface InstancePaths {
  version: string;
  mods: string;
  resourcepacks: string;
  shaderpacks: string;
  saves: string;
  screenshots: string;
  game_dir: string;
}

export interface PackFilePreview {
  path: string;
  size: number;
  url: string | null;
  required: boolean;
}

export interface PackPreview {
  format: string;
  name: string;
  version: string;
  game_version: string;
  loader: string;
  loader_version: string | null;
  detected_parent: string | null;
  file_count: number;
  download_bytes: number;
  override_bytes: number;
  files: PackFilePreview[];
  warnings: string[];
  suggested_id: string;
}

export interface ModpackImportOptions {
  instance_name?: string | null;
  instance_id?: string | null;
  loader_version?: string | null;
  parent_version?: string | null;
  install_optional?: boolean;
}

export interface ModpackImportResult {
  instance_id: string;
  display_name: string;
  installed_files: number;
  failed: string[];
}

export interface CfProject {
  source: "curseforge" | "modrinth" | string;
  id: string;
  name: string;
  slug: string;
  summary: string;
  download_count: number;
  class_id: number;
  logo_url: string;
  author: string;
  updated: string;
  categories: string[];
}

export interface CfSearchResult {
  items: CfProject[];
  total: number;
}

export interface CfFileView {
  id: string;
  name: string;
  file_name: string;
  file_date: string;
  size: number;
  download_url: string | null;
  game_versions: string[];
  release_type: number;
}

export interface AccountView {
  id: string;
  kind: string;
  name: string;
  uuid: string;
  skin_path: string | null;
  skin_model: string;
}

export interface AccountSkin {
  data_url: string;
  model: string;
  source: "local" | "official" | "default";
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
  gameDirs: () => invoke<GameDirsView>("game_dirs"),
  addGameDir: (path: string) => invoke<Config>("add_game_dir", { path }),
  switchGameDir: (path: string) => invoke<Config>("switch_game_dir", { path }),
  removeGameDir: (path: string) => invoke<Config>("remove_game_dir", { path }),
  fetchVersions: (force = false) => invoke<VersionSummary[]>("fetch_versions", { force }),
  installedVersions: () => invoke<InstalledVersion[]>("installed_versions"),
  installVersion: (versionId: string) => invoke<InstalledVersion>("install_version", { versionId }),
  cancelInstall: () => invoke<void>("cancel_install"),
  loaderOptions: (versionId: string) =>
    invoke<LoaderOption[]>("loader_options", { versionId }),
  loaderVersions: (versionId: string, loader: string) =>
    invoke<LoaderVersion[]>("loader_versions", { versionId, loader }),
  installLoader: (versionId: string, loader: string, loaderVersion?: string) =>
    invoke<InstalledVersion>("install_loader", { versionId, loader, loaderVersion }),
  instanceContext: (versionId: string) =>
    invoke<InstanceContext>("instance_context", { versionId }),
  instanceInfo: (versionId: string) =>
    invoke<InstanceInfo>("instance_info", { versionId }),
  instanceMetas: () => invoke<Record<string, InstanceMeta>>("instance_metas"),
  instanceSaveMeta: (versionId: string, meta: InstanceMeta) =>
    invoke<InstanceInfo>("instance_save_meta", { versionId, meta }),
  instanceFiles: (versionId: string, kind: string) =>
    invoke<InstanceFile[]>("instance_files", { versionId, kind }),
  instanceToggleFile: (versionId: string, kind: string, name: string) =>
    invoke<void>("instance_toggle_file", { versionId, kind, name }),
  instanceDeleteFile: (versionId: string, kind: string, name: string) =>
    invoke<void>("instance_delete_file", { versionId, kind, name }),
  instanceImportFiles: (versionId: string, kind: string, paths: string[]) =>
    invoke<number>("instance_import_files", { versionId, kind, paths }),
  instancePaths: (versionId: string) =>
    invoke<InstancePaths>("instance_paths", { versionId }),
  modpackInspect: (path: string) => invoke<PackPreview>("modpack_inspect", { path }),
  modpackImport: (path: string, options: ModpackImportOptions) =>
    invoke<ModpackImportResult>("modpack_import", { path, options }),
  cfSearch: (
    classId: number,
    query: string,
    gameVersion?: string | null,
    loader?: string | null,
    index = 0,
    sortField = 2
  ) =>
    invoke<CfSearchResult>("cf_search", {
      classId,
      query,
      gameVersion: gameVersion ?? null,
      loader: loader ?? null,
      index,
      sortField,
    }),
  cfGameVersions: () => invoke<string[]>("cf_game_versions"),
  cfFiles: (modId: string, gameVersion?: string | null, loader?: string | null) =>
    invoke<CfFileView[]>("cf_files", {
      modId,
      gameVersion: gameVersion ?? null,
      loader: loader ?? null,
    }),
  modrinthSearch: (
    kind: string,
    query: string,
    gameVersion?: string | null,
    loader?: string | null,
    index = 0,
    sortField = 2
  ) =>
    invoke<CfSearchResult>("modrinth_search", {
      kind,
      query,
      gameVersion: gameVersion ?? null,
      loader: loader ?? null,
      index,
      sortField,
    }),
  modrinthFiles: (
    projectId: string,
    gameVersion?: string | null,
    loader?: string | null
  ) =>
    invoke<CfFileView[]>("modrinth_files", {
      projectId,
      gameVersion: gameVersion ?? null,
      loader: loader ?? null,
    }),
  cfInstallFile: (versionId: string, kind: string, url: string, fileName: string) =>
    invoke<string>("cf_install_file", { versionId, kind, url, fileName }),
  deleteVersion: (versionId: string) => invoke<void>("delete_version", { versionId }),
  getAccounts: () => invoke<AccountView[]>("get_accounts"),
  createOfflineAccount: (name: string) => invoke<AccountView>("create_offline_account", { name }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  selectAccount: (id: string) => invoke<void>("select_account", { id }),
  getAccountSkin: (accountId: string) =>
    invoke<AccountSkin>("get_account_skin", { accountId }),
  saveAccountSkin: (accountId: string, pngBase64: string, model: string) =>
    invoke<AccountView>("save_account_skin", { accountId, pngBase64, model }),
  clearAccountSkin: (accountId: string) =>
    invoke<AccountView>("clear_account_skin", { accountId }),
  exportAccountSkin: (accountId: string, targetPath: string) =>
    invoke<void>("export_account_skin", { accountId, targetPath }),
  uploadAccountSkin: (accountId: string, pngBase64: string, model: string) =>
    invoke<AccountView>("upload_account_skin", { accountId, pngBase64, model }),
  msLoginBegin: () => invoke<DeviceCode>("ms_login_begin"),
  detectJava: () => invoke<JavaInfo[]>("detect_java"),
  installJava: (major: number) => invoke<JavaInfo>("install_java", { major }),
  launchVersion: (versionId: string) => invoke<number>("launch_version", { versionId }),
  killGame: () => invoke<void>("kill_game"),
  importBackground: (path: string) =>
    invoke<BackgroundInfo>("import_background", { path }),
  backgroundDataUrl: (path: string) =>
    invoke<string>("background_data_url", { path }),
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

export async function pickImage(): Promise<string | null> {
  const result = await open({
    directory: false,
    multiple: false,
    title: "选择背景图片",
    filters: [
      { name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "bmp", "gif"] },
    ],
  });
  return typeof result === "string" ? result : null;
}

export async function saveSkinDialog(defaultName = "skin.png"): Promise<string | null> {
  const result = await save({
    title: "导出皮肤",
    defaultPath: defaultName,
    filters: [{ name: "Minecraft 皮肤", extensions: ["png"] }],
  });
  return typeof result === "string" ? result : null;
}

export function openFolder(path: string) {
  return openPath(path);
}

export function openExternal(url: string) {
  return openUrl(url);
}
