<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import {
  api,
  pickDirectory,
  pickImage,
  pickSettingsFile,
  saveSettingsDialog,
  type Config,
  type GameDirEntry,
  type UpdateInfo,
  type UpdateProgress,
} from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodSelect from "../components/WoodSelect.vue";
import WoodSlider from "../components/WoodSlider.vue";
import WoodToggle from "../components/WoodToggle.vue";
import {
  applyTheme,
  installJava,
  loadBackground,
  notify,
  refreshConfig,
  refreshInstanceMetas,
  refreshInstalled,
  refreshJava,
  refreshVersions,
  saveConfig,
  startupUpdate,
  store,
} from "../store";

const form = reactive<Config>({
  game_dir: "",
  game_dirs: [],
  recent_servers: [],
  hall_url: "",
  update_url: "",
  update_mode: "notify",
  java_path: null,
  memory_mb: 4096,
  memory_auto: true,
  memory_mode: "auto",
  java_args: "",
  game_args: "",
  pre_launch: "",
  disable_launch_wrapper: false,
  disable_lwjgl_unsafe: false,
  prefer_gpu: false,
  window_width: 1280,
  window_height: 720,
  fullscreen: false,
  download_source: "auto",
  concurrency: 8,
  animations: true,
  version_isolation: true,
  selected_version: null,
  selected_account: null,
  theme: "paper",
  background_image: null,
  background_blur: 18,
  glass_opacity: 78,
  glass_blur: 14,
  background_dim: 0,
  ui_radius: "default",
  ui_density: "normal",
});

/// 从 store 同步到表单期间要抑制自动保存，避免刚打开页面就写盘。
let syncing = false;

watch(
  () => store.config,
  async (config) => {
    if (!config) return;
    syncing = true;
    Object.assign(form, config);
    await nextTick();
    syncing = false;
  },
  { immediate: true }
);

/// 除游戏目录外的设置都即时保存；游戏目录允许手动输入，留给保存按钮校验。
const autoSaveKeys: (keyof Config)[] = [
  "theme",
  "background_image",
  "background_blur",
  "glass_blur",
  "glass_opacity",
  "background_dim",
  "ui_radius",
  "ui_density",
  "animations",
  "hall_url",
  "update_url",
  "update_mode",
  "java_path",
  "memory_mb",
  "memory_auto",
  "memory_mode",
  "java_args",
  "game_args",
  "pre_launch",
  "disable_launch_wrapper",
  "disable_lwjgl_unsafe",
  "prefer_gpu",
  "window_width",
  "window_height",
  "fullscreen",
  "download_source",
  "concurrency",
  "version_isolation",
];

let autoTimer: number | undefined;
let autoSeq = 0;
let autoPending = false;

function autoPatch(): Partial<Config> {
  const patch: Partial<Config> = {};
  for (const key of autoSaveKeys) {
    (patch as Record<string, unknown>)[key] = form[key];
  }
  return patch;
}

async function runAutoSave() {
  if (!store.config) return;
  autoPending = false;
  const seq = autoSeq;
  try {
    const saved = await api.saveConfig({ ...store.config, ...autoPatch() });
    // 保存期间又改过设置就放弃回填，让下一次防抖保存继续写入新值
    if (seq === autoSeq) {
      syncing = true;
      store.config = saved;
      await nextTick();
      syncing = false;
    }
  } catch (err) {
    notify(`自动保存失败：${String(err)}`, "err");
  }
}

watch(
  () => autoSaveKeys.map((key) => form[key]),
  () => {
    if (syncing) return;
    autoSeq += 1;
    autoPending = true;
    window.clearTimeout(autoTimer);
    autoTimer = window.setTimeout(() => void runAutoSave(), 600);
  }
);

const runtimeMajor = ref("21");
const saving = ref(false);
const runtimeBusy = ref(false);

type SettingsTab =
  | "game"
  | "java"
  | "appearance"
  | "performance"
  | "download"
  | "multiplayer"
  | "update";
const settingsTab = ref<SettingsTab>("game");

const settingsTabs: { value: SettingsTab; label: string; icon: string; no: string }[] = [
  { value: "game", label: "游戏与启动", icon: "cube", no: "01" },
  { value: "java", label: "Java 运行时", icon: "database", no: "02" },
  { value: "appearance", label: "外观", icon: "image", no: "03" },
  { value: "performance", label: "性能与参数", icon: "sun", no: "04" },
  { value: "download", label: "下载", icon: "download", no: "05" },
  { value: "multiplayer", label: "联机", icon: "globe", no: "06" },
  { value: "update", label: "更新", icon: "refresh", no: "07" },
];

async function selectTab(tab: SettingsTab) {
  settingsTab.value = tab;
  await nextTick();
  document.querySelector(".main")?.scrollTo({ top: 0, behavior: "smooth" });
}

const themes = [
  { value: "paper", label: "纸木", colors: ["#f4ebdd", "#6f4224", "#5e7b49"] },
  { value: "celadon", label: "青瓷", colors: ["#eef1e8", "#3f6b57", "#7a9b6d"] },
  { value: "terracotta", label: "赤陶", colors: ["#f6e9dc", "#9b4f3a", "#7d8455"] },
  { value: "moon", label: "月白", colors: ["#eef1f6", "#4b5d78", "#5f8892"] },
  { value: "sakura", label: "樱花", colors: ["#fdf1f2", "#b0566e", "#d99aab"] },
  { value: "ocean", label: "深海", colors: ["#eaf2f6", "#2f6076", "#3e8c8c"] },
  { value: "lavender", label: "香芋", colors: ["#f3eff8", "#6d5590", "#b49ccb"] },
  { value: "nocturne", label: "夜墨", colors: ["#1d1a16", "#c08a55", "#88a867"] },
  { value: "graphite", label: "石墨", colors: ["#1b1d20", "#9fb4c7", "#8aa88a"] },
];

const builtinBackgrounds = [
  { value: "forest", label: "林间晨雾" },
  { value: "dune", label: "赤陶沙丘" },
  { value: "night", label: "夜航星空" },
  { value: "sakura", label: "樱花季" },
  { value: "ocean", label: "深海波光" },
  { value: "aurora", label: "极光" },
];

const radiusOptions = [
  { value: "sharp", label: "硬朗直角" },
  { value: "default", label: "标准圆角" },
  { value: "round", label: "柔和圆润" },
];

const densityOptions = [
  { value: "compact", label: "紧凑" },
  { value: "normal", label: "标准" },
  { value: "comfy", label: "宽松" },
];

const memoryModes = [
  { value: "auto", label: "自动（G1GC）" },
  { value: "g1gc", label: "调优 G1GC" },
  { value: "zgc", label: "ZGC（低暂停）" },
  { value: "custom", label: "自定义（不追加 GC 参数）" },
];

const totalMb = computed(() => store.systemMemory.total_mb || 0);
const usedMb = computed(() =>
  Math.max(0, totalMb.value - (store.systemMemory.available_mb || 0))
);
const recommendedMb = computed(() => {
  if (!totalMb.value) return 4096;
  const target = Math.min(8192, Math.max(1024, Math.floor(totalMb.value / 3)));
  return Math.max(1024, Math.floor(target / 512) * 512);
});
const sliderMax = computed(() =>
  totalMb.value
    ? Math.min(32768, Math.max(2048, Math.floor((totalMb.value - 1024) / 512) * 512))
    : 16384
);
const effectiveMemory = computed(() =>
  form.memory_auto ? recommendedMb.value : form.memory_mb
);
const memoryGb = computed(() => (effectiveMemory.value / 1024).toFixed(1));
const usedPercent = computed(() =>
  totalMb.value ? Math.min(100, (usedMb.value / totalMb.value) * 100) : 0
);

function memoryLabel(mb: number) {
  return (mb / 1024).toFixed(1);
}

function setMemoryAuto() {
  form.memory_auto = true;
  form.memory_mb = recommendedMb.value;
}

function setMemoryManual() {
  form.memory_auto = false;
  if (form.memory_mb < 1024) form.memory_mb = recommendedMb.value;
}

function quickMemory(mb: number) {
  form.memory_auto = false;
  form.memory_mb = Math.max(512, Math.round(mb / 512) * 512);
}

watch(
  () => [form.memory_auto, recommendedMb.value],
  () => {
    if (form.memory_auto) form.memory_mb = recommendedMb.value;
  },
  { immediate: true }
);

watch(
  () => form.theme,
  (theme) => {
    if (theme) applyTheme(theme);
  }
);

watch(
  () => [form.background_blur, form.glass_blur, form.glass_opacity],
  () => {
    if (!store.config) return;
    store.config.background_blur = form.background_blur;
    store.config.glass_blur = form.glass_blur;
    store.config.glass_opacity = form.glass_opacity;
  }
);

watch(
  () => [form.background_dim, form.ui_radius, form.ui_density],
  () => {
    if (!store.config) return;
    store.config.background_dim = form.background_dim;
    store.config.ui_radius = form.ui_radius;
    store.config.ui_density = form.ui_density;
  }
);

async function chooseBackground() {
  const path = await pickImage();
  if (!path) return;
  try {
    const info = await api.importBackground(path);
    store.backgroundPreset = "";
    store.backgroundUrl = info.data_url;
    form.background_image = info.path;
    notify("背景已应用并自动保存");
  } catch (err) {
    notify(`设置背景失败：${String(err)}`, "err");
  }
}

function previewBuiltin(name: string) {
  store.backgroundPreset = name;
  store.backgroundUrl = "";
  form.background_image = `builtin:${name}`;
  // 旧默认值偏保守，首次选择内置底图时切到更能看出磨砂质感的一组参数
  if (form.background_blur === 18) form.background_blur = 4;
  if (form.glass_blur === 14) form.glass_blur = 18;
  if (form.glass_opacity === 78) form.glass_opacity = 62;
}

function clearBackground() {
  store.backgroundUrl = "";
  store.backgroundPreset = "";
  form.background_image = null;
  notify("已移除背景");
}

const sourceOptions = [
  { value: "auto", label: "自动（优先镜像）" },
  { value: "mirror", label: "仅 BMClAPI 镜像" },
  { value: "official", label: "仅官方源" },
];

const javaOptions = [
  { value: "8", label: "Java 8" },
  { value: "17", label: "Java 17" },
  { value: "21", label: "Java 21" },
  { value: "25", label: "Java 25" },
];

const updateModes = [
  { value: "auto", label: "有新版本时自动下载" },
  { value: "notify", label: "有新版本时提示" },
  { value: "off", label: "关闭自动检查" },
];

async function chooseDir() {
  const dir = await pickDirectory();
  if (dir) form.game_dir = dir;
}

const gameDirs = ref<GameDirEntry[]>([]);
const activeDir = ref("");
const officialDir = ref("");
const dirBusy = ref(false);

onMounted(loadDirs);

const appVersion = ref("");
const updateInfo = ref<UpdateInfo | null>(null);
const updateBusy = ref(false);
const updateError = ref("");
const updatePath = ref("");
const updateProgress = ref<UpdateProgress | null>(null);
let stopUpdateProgress: (() => void) | null = null;

const updatePercent = computed(() => {
  const progress = updateProgress.value;
  if (!progress || !progress.total) return 0;
  return Math.min(100, Math.round((progress.downloaded / progress.total) * 100));
});

const updateProgressText = computed(() => {
  const progress = updateProgress.value;
  if (!progress) return "";
  if (progress.total) return `${updatePercent.value}%`;
  return `${(progress.downloaded / 1048576).toFixed(1)} MB`;
});

async function initUpdateSection() {
  try {
    appVersion.value = await api.appVersion();
  } catch {
    appVersion.value = "";
  }
  // 启动时已经检查到的新版本直接接续展示，不用再点一次检查
  if (startupUpdate.value) {
    updateInfo.value = startupUpdate.value.info;
    updatePath.value = startupUpdate.value.path ?? "";
  }
  stopUpdateProgress = await listen<UpdateProgress>("update-progress", (event) => {
    updateProgress.value = event.payload;
  });
}

onMounted(() => {
  void initUpdateSection();
});

onBeforeUnmount(() => {
  window.clearTimeout(autoTimer);
  if (autoPending) void runAutoSave();
  stopUpdateProgress?.();
});

async function checkUpdate() {
  if (updateBusy.value) return;
  startupUpdate.value = null;
  updateBusy.value = true;
  updateError.value = "";
  updateInfo.value = null;
  updatePath.value = "";
  updateProgress.value = null;
  try {
    updateInfo.value = await api.updateCheck(form.update_url);
  } catch (err) {
    updateError.value = String(err);
  } finally {
    updateBusy.value = false;
  }
}

async function downloadUpdate() {
  if (!updateInfo.value || updateBusy.value) return;
  updateBusy.value = true;
  updateError.value = "";
  updatePath.value = "";
  updateProgress.value = { downloaded: 0, total: 0 };
  try {
    updatePath.value = await api.updateDownload(
      updateInfo.value.url,
      updateInfo.value.latest
    );
  } catch (err) {
    updateError.value = String(err);
    updateProgress.value = null;
  } finally {
    updateBusy.value = false;
  }
}

async function installUpdate() {
  if (!updatePath.value) return;
  try {
    await api.updateInstall(updatePath.value);
  } catch (err) {
    updateError.value = String(err);
  }
}

async function loadDirs() {
  try {
    const view = await api.gameDirs();
    gameDirs.value = view.dirs;
    activeDir.value = view.active;
    officialDir.value = view.official;
  } catch {
    gameDirs.value = [];
  }
}

async function adoptConfig(config: Config) {
  syncing = true;
  store.config = config;
  Object.assign(form, config);
  await nextTick();
  syncing = false;
  activeDir.value = config.game_dir;
  await refreshInstalled();
  await refreshVersions(true);
  await refreshInstanceMetas();
  await loadDirs();
  if (
    config.selected_version &&
    !store.installed.some((item) => item.id === config.selected_version)
  ) {
    await saveConfig({ selected_version: store.installed[0]?.id ?? null });
  }
}

async function switchDir(path: string) {
  if (dirBusy.value || path === activeDir.value) return;
  dirBusy.value = true;
  try {
    await adoptConfig(await api.switchGameDir(path));
    notify(`已切换到 ${path}`);
  } catch (err) {
    notify(`切换目录失败：${String(err)}`, "err");
  } finally {
    dirBusy.value = false;
  }
}

async function addDir() {
  const path = await pickDirectory();
  if (!path) return;
  dirBusy.value = true;
  try {
    await adoptConfig(await api.addGameDir(path));
    notify(`已添加并切换到 ${path}`);
  } catch (err) {
    notify(`添加目录失败：${String(err)}`, "err");
  } finally {
    dirBusy.value = false;
  }
}

async function useOfficialDir() {
  if (!officialDir.value) return;
  dirBusy.value = true;
  try {
    await adoptConfig(await api.addGameDir(officialDir.value));
    notify("已切换到官方启动器目录");
  } catch (err) {
    notify(`切换目录失败：${String(err)}`, "err");
  } finally {
    dirBusy.value = false;
  }
}

async function removeDir(path: string) {
  dirBusy.value = true;
  try {
    await adoptConfig(await api.removeGameDir(path));
    notify("已从目录列表移除，磁盘文件未删除");
  } catch (err) {
    notify(`移除目录失败：${String(err)}`, "err");
  } finally {
    dirBusy.value = false;
  }
}

async function rescanJava() {
  await refreshJava();
  notify(`检测到 ${store.javaList.length} 个 Java`);
}

async function installRuntime() {
  if (runtimeBusy.value) return;
  runtimeBusy.value = true;
  try {
    await installJava(Number(runtimeMajor.value));
  } finally {
    runtimeBusy.value = false;
  }
}

const backupBusy = ref(false);

async function exportSettings() {
  if (backupBusy.value) return;
  const path = await saveSettingsDialog();
  if (!path) return;
  backupBusy.value = true;
  try {
    await api.exportSettings(path);
    notify("设置已导出");
  } catch (err) {
    notify(`导出设置失败：${String(err)}`, "err");
  } finally {
    backupBusy.value = false;
  }
}

async function importSettings() {
  const path = await pickSettingsFile();
  if (!path) return;
  backupBusy.value = true;
  window.clearTimeout(autoTimer);
  autoPending = false;
  try {
    const config = await api.importSettings(path);
    await adoptConfig(config);
    await loadBackground();
    notify("设置已导入并生效");
  } catch (err) {
    notify(`导入设置失败：${String(err)}`, "err");
  } finally {
    backupBusy.value = false;
  }
}

async function save() {
  if (!form.game_dir.trim()) {
    notify("请先选择游戏目录", "err");
    return;
  }
  window.clearTimeout(autoTimer);
  autoPending = false;
  saving.value = true;
  try {
    await saveConfig({ ...form });
    await loadBackground();
    await loadDirs();
    notify("设置已保存");
  } finally {
    saving.value = false;
  }
}

function useAutoJava() {
  form.java_path = null;
  notify("已改为自动选择 Java");
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <span class="eyebrow">DOCL / SETTINGS</span>
        <div class="muted small">除游戏目录外，修改会自动保存</div>
      </div>
      <WoodButton variant="primary" sheen :disabled="saving" @click="save">
        <AppIcon name="check" :size="15" />
        {{ saving ? "保存中…" : "保存设置" }}
      </WoodButton>
    </header>

    <div class="settings-layout">
      <aside class="settings-side card">
        <button
          v-for="tab in settingsTabs"
          :key="tab.value"
          class="settings-side__item"
          :class="{ 'settings-side__item--active': settingsTab === tab.value }"
          @click="selectTab(tab.value)"
        >
          <AppIcon :name="tab.icon" :size="16" />
          <span class="settings-side__label">{{ tab.label }}</span>
          <span class="settings-side__no mono">{{ tab.no }}</span>
        </button>
      </aside>

      <div class="settings-main">
    <section v-if="settingsTab === 'game'" class="card card-pad group reveal">
      <div class="group-head">
        <span class="group-no display">01</span>
        <div class="group-titles">
          <span class="eyebrow">GAME</span>
          <div class="section-title">游戏与启动</div>
        </div>
        <span class="group-line" />
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>游戏目录</strong>
          <span>versions / assets / libraries 都放在这里</span>
        </div>
        <div class="setting__control setting__control--wide">
          <div class="row">
            <WoodInput v-model="form.game_dir" mono />
            <IconButton icon="folder" label="选择游戏目录" :size="38" @click="chooseDir" />
          </div>
        </div>
      </div>
      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>游戏目录列表</strong>
          <span>可以保存多个 .minecraft 文件夹，随时切换</span>
        </div>
        <div class="setting__control setting__control--wide">
          <div class="dir-list">
            <div
              v-for="dir in gameDirs"
              :key="dir.path"
              class="dir-row"
              :class="{ 'dir-row--active': dir.path === activeDir }"
            >
              <button class="dir-row__main" @click="switchDir(dir.path)">
                <AppIcon name="folder" :size="15" />
                <span class="dir-row__text">
                  <strong>{{ dir.name }}</strong>
                  <span class="mono">{{ dir.path }}</span>
                </span>
                <span v-if="dir.path === activeDir" class="tag tag--grass">当前</span>
              </button>
              <IconButton
                icon="trash"
                label="从列表移除"
                :size="30"
                :disabled="gameDirs.length <= 1 || dirBusy"
                @click="removeDir(dir.path)"
              />
            </div>
          </div>
          <div class="row dir-actions">
            <WoodButton variant="soft" size="sm" :disabled="dirBusy" @click="addDir">
              <AppIcon name="plus" :size="14" />
              添加已有文件夹
            </WoodButton>
            <WoodButton
              variant="ghost"
              size="sm"
              :disabled="dirBusy || !officialDir"
              @click="useOfficialDir"
            >
              官方启动器目录
            </WoodButton>
          </div>
        </div>
      </div>
      <div class="setting">
        <div class="setting__label">
          <strong>版本隔离</strong>
          <span>每个版本使用独立的游戏目录，存档互不影响</span>
        </div>
        <div class="setting__control">
          <WoodToggle v-model="form.version_isolation" />
        </div>
      </div>
      <div class="setting">
        <div class="setting__label">
          <strong>启动窗口尺寸</strong>
          <span>游戏内分辨率，按需调整</span>
        </div>
        <div class="setting__control">
          <div class="row resolution">
            <WoodInput
              :model-value="String(form.window_width)"
              placeholder="宽"
              @update:model-value="form.window_width = Number($event) || 1280"
            />
            <span class="muted resolution__x">×</span>
            <WoodInput
              :model-value="String(form.window_height)"
              placeholder="高"
              @update:model-value="form.window_height = Number($event) || 720"
            />
          </div>
        </div>
      </div>
      <div class="setting setting--last">
        <div class="setting__label">
          <strong>配置备份</strong>
          <span>导出为 JSON 文件，可在另一台电脑导入；不含账号、皮肤与存档</span>
        </div>
        <div class="setting__control">
          <div class="row">
            <WoodButton variant="soft" size="sm" :disabled="backupBusy" @click="exportSettings">
              导出设置
            </WoodButton>
            <WoodButton variant="ghost" size="sm" :disabled="backupBusy" @click="importSettings">
              导入设置
            </WoodButton>
          </div>
        </div>
      </div>
    </section>

    <section v-if="settingsTab === 'java'" class="card card-pad group reveal" style="--i: 1">
      <div class="group-head">
        <span class="group-no display">02</span>
        <div class="group-titles">
          <span class="eyebrow">RUNTIME</span>
          <div class="section-title">Java 运行时</div>
        </div>
        <span class="group-line" />
        <WoodButton variant="ghost" size="sm" @click="rescanJava">
          <AppIcon name="refresh" :size="14" />
          重新检测
        </WoodButton>
      </div>

      <p class="muted small">
        启动器会按版本要求自动挑选 Java；手动选择后将优先使用指定路径。
      </p>

      <div class="java-list">
        <button
          v-for="java in store.javaList"
          :key="java.path"
          class="java-item"
          :class="{ 'java-item--active': form.java_path === java.path }"
          @click="form.java_path = java.path"
        >
          <span class="java-item__major display">Java {{ java.major }}</span>
          <span class="java-item__path mono">{{ java.path }}</span>
          <AppIcon
            v-if="form.java_path === java.path"
            name="check"
            :size="16"
            class="java-item__check"
          />
          <span class="tag">{{ java.source }}</span>
        </button>
        <div v-if="!store.javaList.length" class="java-empty">
          未检测到 Java。可以在下方直接下载官方运行时。
        </div>
      </div>

      <div class="row java-actions">
        <WoodButton variant="ghost" size="sm" @click="useAutoJava">
          <AppIcon name="refresh" :size="14" />
          恢复自动选择
        </WoodButton>
        <span class="muted small" v-if="form.java_path">指定：{{ form.java_path }}</span>
        <span class="muted small" v-else>当前：自动选择</span>
      </div>

      <div class="divider" />

      <div class="setting setting--last">
        <div class="setting__label">
          <strong>下载官方 Java 运行时</strong>
          <span>由 Mojang 提供，安装到启动器数据目录，不影响系统环境</span>
        </div>
        <div class="setting__control">
          <div class="row">
            <WoodSelect v-model="runtimeMajor" :options="javaOptions" />
            <WoodButton
              variant="soft"
              size="sm"
              class="runtime-action"
              :disabled="runtimeBusy"
              @click="installRuntime"
            >
              <AppIcon name="download" :size="14" />
              {{ runtimeBusy ? "安装中…" : "下载并安装" }}
            </WoodButton>
          </div>
        </div>
      </div>
    </section>

    <section
      v-if="settingsTab === 'appearance'"
      class="card card-pad group reveal"
      :style="{ '--i': 2 }"
    >
      <div class="group-head">
        <span class="group-no display">03</span>
        <div class="group-titles">
          <span class="eyebrow">APPEARANCE</span>
          <div class="section-title">外观</div>
        </div>
        <span class="group-line" />
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>主题配色</strong>
          <span>即时预览并自动保存</span>
        </div>
        <div class="setting__control">
          <div class="theme-grid">
          <button
            v-for="theme in themes"
            :key="theme.value"
            class="theme-chip"
            :class="{ 'theme-chip--active': form.theme === theme.value }"
            @click="form.theme = theme.value"
          >
            <span class="theme-chip__dots">
              <i v-for="color in theme.colors" :key="color" :style="{ background: color }" />
            </span>
            <span class="theme-chip__label">{{ theme.label }}</span>
            <AppIcon
              v-if="form.theme === theme.value"
              name="check"
              :size="14"
              class="theme-chip__check"
            />
          </button>
          </div>
        </div>
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>内置底图</strong>
          <span>点选即可预览磨砂玻璃效果，无需本地图片</span>
        </div>
        <div class="setting__control">
          <div class="bg-presets">
            <button
              v-for="bg in builtinBackgrounds"
              :key="bg.value"
              class="bg-chip"
              :class="{
                'bg-chip--active': form.background_image === `builtin:${bg.value}`,
              }"
              @click="previewBuiltin(bg.value)"
            >
              <span class="bg-chip__swatch" :class="`bg-chip__swatch--${bg.value}`" />
              {{ bg.label }}
            </button>
            <button
              class="bg-chip"
              :class="{ 'bg-chip--active': !form.background_image }"
              @click="clearBackground"
            >
              <span class="bg-chip__swatch bg-chip__swatch--none" />
              无背景
            </button>
          </div>
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>背景图片</strong>
          <span>PNG / JPG / WEBP / BMP / GIF，上限 24 MB</span>
        </div>
        <div class="setting__control">
          <div class="row">
            <span class="muted small">{{ form.background_image && !form.background_image.startsWith("builtin:") ? "已设置" : "未设置" }}</span>
            <IconButton icon="image" label="选择背景图片" :size="34" @click="chooseBackground" />
            <IconButton
              icon="trash"
              label="移除背景图片"
              variant="danger"
              :size="34"
              :disabled="!form.background_image"
              @click="clearBackground"
            />
          </div>
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>背景模糊</strong>
          <span>{{ form.background_blur }} px，越大越柔和</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.background_blur" :min="0" :max="30" :step="1" />
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>玻璃模糊</strong>
          <span>{{ form.glass_blur }} px，磨砂面板的雾化程度</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.glass_blur" :min="0" :max="24" :step="1" />
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>玻璃不透明度</strong>
          <span>{{ form.glass_opacity }}%，越低越透出背景</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.glass_opacity" :min="30" :max="95" :step="1" />
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>背景遮罩</strong>
          <span>{{ form.background_dim }}%，压暗底图让内容更清晰</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.background_dim" :min="0" :max="60" :step="2" />
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>界面圆角</strong>
          <span>卡片与面板的边角风格</span>
        </div>
        <div class="setting__control">
          <WoodSelect v-model="form.ui_radius" :options="radiusOptions" />
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>界面密度</strong>
          <span>控制间距与留白，小窗口可用紧凑</span>
        </div>
        <div class="setting__control">
          <WoodSelect v-model="form.ui_density" :options="densityOptions" />
        </div>
      </div>

      <div class="setting setting--last">
        <div class="setting__label">
          <strong>界面动效</strong>
          <span>关闭后页面切换与控件过渡立即完成</span>
        </div>
        <div class="setting__control">
          <WoodToggle v-model="form.animations" />
        </div>
      </div>
    </section>

    <section
      v-if="settingsTab === 'performance'"
      class="card card-pad group reveal"
      :style="{ '--i': 3 }"
    >
      <div class="group-head">
        <span class="group-no display">04</span>
        <div class="group-titles">
          <span class="eyebrow">PERFORMANCE</span>
          <div class="section-title">性能与参数</div>
        </div>
        <span class="group-line" />
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>内存分配</strong>
          <span>自动按物理内存推荐，也可以切到自定义手动拖动</span>
        </div>
        <div class="setting__control setting__control--wide">
          <div class="memory">
            <div class="memory__head">
              <div class="memory__modes">
                <button
                  class="memory__mode"
                  :class="{ 'memory__mode--active': form.memory_auto }"
                  @click="setMemoryAuto"
                >
                  自动配置
                </button>
                <button
                  class="memory__mode"
                  :class="{ 'memory__mode--active': !form.memory_auto }"
                  @click="setMemoryManual"
                >
                  自定义
                </button>
              </div>
              <span class="memory__alloc display">{{ memoryGb }} GB</span>
            </div>
            <div class="memory__slider" :class="{ 'memory__slider--off': form.memory_auto }">
              <WoodSlider v-model="form.memory_mb" :min="1024" :max="sliderMax" :step="512" />
            </div>
            <div class="memory__quick">
              <button class="memory__chip" @click="quickMemory(2048)">2 GB</button>
              <button class="memory__chip" @click="quickMemory(4096)">4 GB</button>
              <button class="memory__chip" @click="quickMemory(6144)">6 GB</button>
              <button class="memory__chip" @click="quickMemory(8192)">8 GB</button>
              <button
                class="memory__chip"
                :disabled="!totalMb"
                @click="quickMemory(totalMb / 2)"
              >
                半内存
              </button>
            </div>
            <div class="memory__bar">
              <span class="memory__bar-fill" :style="{ width: `${usedPercent}%` }" />
            </div>
            <div class="memory__legend">
              <span>已使用 {{ memoryLabel(usedMb) }} GB / {{ memoryLabel(totalMb) }} GB</span>
              <span>游戏分配 {{ memoryGb }} GB</span>
            </div>
          </div>
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>内存管理</strong>
          <span>决定追加的垃圾回收参数，自定义时不追加</span>
        </div>
        <div class="setting__control">
          <WoodSelect v-model="form.memory_mode" :options="memoryModes" />
        </div>
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>Java 虚拟机参数</strong>
          <span>追加在内存参数之后，空格分隔；可用引号包住带空格的路径</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodInput
            v-model="form.java_args"
            mono
            placeholder="-XX:-OmitStackTraceInFastThrow -Dfml.ignorePatchDiscrepancies=true"
          />
        </div>
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>游戏参数</strong>
          <span>追加到游戏主类之后，例如 --width 1600 --height 900</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodInput v-model="form.game_args" mono placeholder="--width 1600 --height 900" />
        </div>
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>启动前执行命令</strong>
          <span>工作目录为游戏目录，命中退出码不影响继续启动</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodInput v-model="form.pre_launch" mono placeholder="例如 java -jar packwiz-installer-bootstrap.jar ..." />
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>禁用 Java Launch Wrapper</strong>
          <span>不再向游戏注入 DOCL 启动器标识，部分反作弊需要关闭</span>
        </div>
        <div class="setting__control">
          <WoodToggle v-model="form.disable_launch_wrapper" />
        </div>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>禁用 LWJGL Unsafe Agent</strong>
          <span>Java 24+ 默认放行 LWJGL 的 Unsafe 内存访问，关闭后可能告警或崩溃</span>
        </div>
        <div class="setting__control">
          <WoodToggle v-model="form.disable_lwjgl_unsafe" />
        </div>
      </div>

      <div class="setting setting--last">
        <div class="setting__label">
          <strong>使用高性能显卡</strong>
          <span>为当前 Java 写入 Windows 显卡偏好，下次启动生效</span>
        </div>
        <div class="setting__control">
          <WoodToggle v-model="form.prefer_gpu" />
        </div>
      </div>
    </section>

    <section
      v-if="settingsTab === 'download'"
      class="card card-pad group reveal"
      :style="{ '--i': 4 }"
    >
      <div class="group-head">
        <span class="group-no display">05</span>
        <div class="group-titles">
          <span class="eyebrow">DOWNLOAD</span>
          <div class="section-title">下载</div>
        </div>
        <span class="group-line" />
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>下载源</strong>
          <span>镜像失败时会自动回退到官方源</span>
        </div>
        <div class="setting__control">
          <WoodSelect v-model="form.download_source" :options="sourceOptions" />
        </div>
      </div>
      <div class="setting setting--last">
        <div class="setting__label">
          <strong>并发下载数</strong>
          <span>当前 {{ form.concurrency }}；网络不稳定时建议降低</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.concurrency" :min="1" :max="16" :step="1" />
        </div>
      </div>
    </section>

    <section
      v-if="settingsTab === 'multiplayer'"
      class="card card-pad group reveal"
      :style="{ '--i': 5 }"
    >
      <div class="group-head">
        <span class="group-no display">06</span>
        <div class="group-titles">
          <span class="eyebrow">MULTIPLAYER</span>
          <div class="section-title">联机</div>
        </div>
        <span class="group-line" />
      </div>

      <div class="setting setting--stack setting--last">
        <div class="setting__label">
          <strong>房间大厅地址</strong>
          <span>留空关闭大厅；填写后可返回 JSON 列表的地址，启动器会按此拉取房间</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodInput
            v-model="form.hall_url"
            mono
            placeholder="https://example.com/docl-rooms.json"
          />
        </div>
      </div>

      <p class="muted small settings-hall-hint">
        支持 <span class="mono">{ "rooms": [ ... ] }</span> 或直接返回数组；每个房间至少包含
        <span class="mono">room</span>（陶瓦邀请码），可选 name、motd、players、max_players、version、updated（Unix 秒）。
      </p>
    </section>

    <section
      v-if="settingsTab === 'update'"
      class="card card-pad group reveal"
      :style="{ '--i': 6 }"
    >
      <div class="group-head">
        <span class="group-no display">07</span>
        <div class="group-titles">
          <span class="eyebrow">UPDATE</span>
          <div class="section-title">启动器更新</div>
        </div>
        <span class="group-line" />
        <span class="tag mono">v{{ appVersion || "—" }}</span>
      </div>

      <div class="setting">
        <div class="setting__label">
          <strong>启动器更新</strong>
          <span>启动时按此策略检查；关闭后仍可在这里手动检查</span>
        </div>
        <div class="setting__control">
          <WoodSelect v-model="form.update_mode" :options="updateModes" />
        </div>
      </div>

      <div class="setting setting--stack">
        <div class="setting__label">
          <strong>更新清单地址</strong>
          <span>留空则不检查更新；填写一个返回版本信息的 JSON 地址</span>
        </div>
        <div class="setting__control setting__control--wide">
          <div class="row">
            <WoodInput
              v-model="form.update_url"
              mono
              placeholder="https://example.com/docl-update.json"
            />
            <WoodButton
              variant="soft"
              size="sm"
              :disabled="updateBusy || !form.update_url.trim()"
              @click="checkUpdate"
            >
              <AppIcon name="refresh" :size="14" />
              {{ updateBusy ? "检查中…" : "检查更新" }}
            </WoodButton>
          </div>
        </div>
      </div>

      <p class="muted small update-hint">
        清单格式：
        <span class="mono">{ "version": "0.2.0", "url": "https://…/DOCL_0.2.0-setup.exe", "notes": "更新说明" }</span>。
        发现新版本后下载的是完整安装包，点击安装时启动器会自动退出。
      </p>

      <div v-if="updateError" class="update-error">
        <AppIcon name="alert" :size="14" />
        {{ updateError }}
        <WoodButton variant="ghost" size="sm" @click="checkUpdate">重试</WoodButton>
      </div>

      <div v-else-if="updateInfo" class="update-panel">
        <div class="update-panel__head">
          <strong>
            {{ updateInfo.available ? `发现新版本 ${updateInfo.latest}` : "已经是最新版本" }}
          </strong>
          <span class="muted small mono">当前 v{{ updateInfo.current }}</span>
        </div>
        <p v-if="updateInfo.notes" class="update-notes">{{ updateInfo.notes }}</p>
        <div v-if="updateInfo.available" class="update-actions">
          <div v-if="updateProgress" class="update-progress">
            <span class="update-progress__bar">
              <span :style="{ width: `${updatePercent}%` }" />
            </span>
            <span class="muted small mono">{{ updateProgressText }}</span>
          </div>
          <WoodButton
            v-if="!updatePath"
            variant="primary"
            size="sm"
            :disabled="updateBusy"
            @click="downloadUpdate"
          >
            <AppIcon name="download" :size="14" />
            {{ updateBusy ? "下载中…" : "下载安装包" }}
          </WoodButton>
          <WoodButton v-else variant="primary" size="sm" @click="installUpdate">
            <AppIcon name="check" :size="14" />
            安装并退出
          </WoodButton>
        </div>
      </div>
    </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-hall-hint {
  margin: 0;
  line-height: 1.6;
}

.update-hint {
  margin: 0;
  line-height: 1.7;
}

.update-error {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid var(--line-strong);
  background: var(--danger-soft);
  color: var(--danger);
  font-size: 12px;
}

.update-error .btn {
  margin-left: auto;
}

.update-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--panel-2);
}

.update-panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.update-panel__head strong {
  font-size: 13px;
  color: var(--ink);
}

.update-notes {
  margin: 0;
  font-size: 12px;
  color: var(--ink-soft);
  line-height: 1.6;
  white-space: pre-wrap;
}

.update-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.update-progress {
  flex: 1;
  min-width: 180px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.update-progress__bar {
  flex: 1;
  height: 6px;
  border-radius: 99px;
  background: rgba(var(--accent-rgb), 0.12);
  overflow: hidden;
}

.update-progress__bar > span {
  display: block;
  height: 100%;
  border-radius: 99px;
  background: linear-gradient(90deg, var(--moss), var(--walnut-light));
  transition: width 0.2s ease;
}

.settings-layout {
  display: grid;
  grid-template-columns: 190px minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}

.settings-side {
  padding: 12px 10px;
  display: flex;
  flex-direction: column;
  gap: 3px;
  position: sticky;
  top: 0;
}

.settings-side__item {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 38px;
  padding: 0 10px;
  border-radius: 8px;
  color: var(--ink-soft);
  font-size: 12.5px;
  font-weight: 600;
  text-align: left;
  transition: background var(--dur-2) ease, color var(--dur-2) ease;
}

.settings-side__item:hover {
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--walnut-deep);
}

.settings-side__item--active {
  background: var(--panel);
  color: var(--walnut-deep);
  box-shadow: var(--shadow-1);
}

.settings-side__item--active .icon {
  color: var(--moss);
}

.settings-side__label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.settings-side__no {
  font-size: 9px;
  color: var(--muted);
  letter-spacing: 0.08em;
}

.settings-main {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

.group-head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 6px;
}

.group-no {
  font-size: 28px;
  line-height: 1;
  color: var(--line-strong);
  min-width: 34px;
}

.group-titles {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.group-line {
  flex: 1;
  height: 1px;
  background: linear-gradient(90deg, var(--line), transparent);
  margin: 0 4px;
}

.setting {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 14px 0;
  border-bottom: 1px dashed var(--line);
}

.setting--last {
  border-bottom: none;
  padding-bottom: 2px;
}

/* 目录这类内容较长的设置改为上下布局，避免左右互相挤压 */
.setting--stack {
  flex-direction: column;
  align-items: stretch;
  gap: 10px;
}

.setting--stack .setting__control {
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  width: 100%;
  max-width: none;
}

.setting--stack .row {
  width: 100%;
}

.setting--stack .row .input {
  flex: 1;
  min-width: 0;
}

.setting__label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.setting__label strong {
  font-size: 13.5px;
  color: var(--ink);
}

.setting__label span {
  font-size: 12px;
  color: var(--muted);
}

.setting__control {
  flex-shrink: 0;
  min-width: 150px;
  display: flex;
  justify-content: flex-end;
}

.setting__control--wide {
  flex: 1;
  max-width: 430px;
}

.resolution {
  width: 230px;
}

.memory {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.memory__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.memory__modes {
  display: flex;
  gap: 3px;
  padding: 3px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: var(--panel-2);
}

.memory__mode {
  height: 28px;
  padding: 0 13px;
  border-radius: 7px;
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 600;
  transition: background var(--dur-1) ease, color var(--dur-1) ease,
    box-shadow var(--dur-1) ease;
}

.memory__mode:hover {
  color: var(--walnut-deep);
}

.memory__mode--active {
  background: var(--panel);
  color: var(--walnut-deep);
  box-shadow: var(--shadow-1);
}

.memory__alloc {
  font-size: 24px;
  line-height: 1;
  color: var(--walnut-deep);
}

.memory__slider--off {
  opacity: 0.55;
  pointer-events: none;
}

.memory__quick {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.memory__chip {
  height: 26px;
  padding: 0 10px;
  border-radius: 7px;
  border: 1px dashed var(--line-strong);
  color: var(--ink-soft);
  font-size: 11.5px;
  font-weight: 600;
  transition: border-color var(--dur-1) ease, color var(--dur-1) ease;
}

.memory__chip:hover:not(:disabled) {
  border-color: var(--moss);
  color: var(--moss);
}

.memory__chip:disabled {
  opacity: 0.4;
  cursor: default;
}

.memory__bar {
  height: 8px;
  border-radius: 99px;
  border: 1px solid var(--line);
  background: var(--paper-2);
  overflow: hidden;
}

.memory__bar-fill {
  display: block;
  height: 100%;
  border-radius: 99px;
  background: linear-gradient(90deg, var(--moss), var(--moss-bright));
  transition: width var(--dur-3) var(--ease-out-expo);
}

.memory__legend {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 11.5px;
  color: var(--muted);
}

.resolution__x {
  font-size: 13px;
}

.dir-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  width: 100%;
}

.dir-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 4px 2px 2px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  transition: border-color var(--dur-2) ease;
}

.dir-row:hover {
  border-color: var(--walnut-light);
}

.dir-row--active {
  border-color: var(--moss);
  background: linear-gradient(180deg, var(--panel), var(--moss-soft));
}

.dir-row__main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 9px;
  height: 42px;
  padding: 0 9px;
  border-radius: 7px;
  text-align: left;
  color: var(--ink-soft);
}

.dir-row__text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.dir-row__text strong {
  font-size: 12.5px;
  color: var(--walnut-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-row__text span {
  font-size: 10.5px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-actions {
  margin-top: 8px;
}

.java-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 12px 0;
}

.java-item {
  display: flex;
  align-items: center;
  gap: 13px;
  padding: 10px 13px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  text-align: left;
  position: relative;
  overflow: hidden;
  transition: transform var(--dur-2) var(--ease-out-expo),
    border-color var(--dur-2) ease, box-shadow var(--dur-2) ease;
}

.java-item:hover {
  transform: translateY(-1px);
  border-color: var(--walnut-light);
}

.java-item--active {
  border-color: var(--moss);
  box-shadow: 0 0 0 3px rgba(94, 123, 73, 0.1);
}

.java-item--active::after {
  content: "";
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--moss);
}

.java-item__major {
  font-size: 18px;
  color: var(--walnut-deep);
  min-width: 84px;
}

.java-item__path {
  flex: 1;
  font-size: 11.5px;
  color: var(--ink-soft);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.java-item__check {
  color: var(--moss);
}

.java-empty {
  padding: 18px;
  border-radius: 10px;
  background: var(--panel-2);
  border: 1px dashed var(--line-strong);
  color: var(--muted);
  font-size: 12.5px;
  text-align: center;
}

.java-actions {
  justify-content: space-between;
}

.java-progress {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 10px;
}

.runtime-action {
  width: 118px;
}

.theme-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  width: 100%;
}

.theme-chip {
  display: flex;
  align-items: center;
  gap: 7px;
  height: 34px;
  padding: 0 10px;
  border-radius: 9px;
  border: 1px solid var(--line);
  background: var(--panel-2);
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 600;
  transition: border-color var(--dur-2) ease, background var(--dur-2) ease,
    transform var(--dur-2) var(--ease-out-expo);
}

.theme-chip:hover {
  border-color: var(--walnut-light);
  transform: translateY(-1px);
}

.theme-chip--active {
  border-color: var(--moss);
  background: var(--moss-soft);
  color: var(--moss);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
}

.theme-chip__dots {
  display: flex;
  gap: 2px;
}

.theme-chip__dots i {
  width: 10px;
  height: 10px;
  border-radius: 3px;
  border: 1px solid rgba(0, 0, 0, 0.12);
}

.theme-chip__check {
  color: var(--moss);
}

.bg-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  width: 100%;
}

.bg-chip {
  display: flex;
  align-items: center;
  gap: 7px;
  height: 34px;
  padding: 0 10px;
  border-radius: 9px;
  border: 1px solid var(--line);
  background: var(--panel-2);
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 600;
  transition: border-color var(--dur-2) ease, transform var(--dur-2) var(--ease-out-expo);
}

.bg-chip:hover {
  border-color: var(--walnut-light);
  transform: translateY(-1px);
}

.bg-chip--active {
  border-color: var(--moss);
  color: var(--moss);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
}

.bg-chip__swatch {
  width: 22px;
  height: 16px;
  border-radius: 4px;
  border: 1px solid rgba(0, 0, 0, 0.14);
  background-size: cover;
}

.bg-chip__swatch--forest {
  background-image: linear-gradient(180deg, #b7ccb0 0 40%, #7d9a6b 41% 60%, #45603c 61%);
}

.bg-chip__swatch--dune {
  background-image: linear-gradient(180deg, #f7dfc0 0 40%, #d9a97f 41% 60%, #a8623f 61%);
}

.bg-chip__swatch--night {
  background-image: linear-gradient(180deg, #141d2e 0 46%, #1e2c42 47% 72%, #16202e 73%);
}

.bg-chip__swatch--sakura {
  background-image: linear-gradient(180deg, #fdeef2 0 38%, #f8d7e1 39% 66%, #e6a3ba 67%);
}

.bg-chip__swatch--ocean {
  background-image: linear-gradient(180deg, #dff1f7 0 36%, #5f9cb2 37% 66%, #2f6076 67%);
}

.bg-chip__swatch--aurora {
  background-image: radial-gradient(circle at 30% 25%, #78eabe 0 18%, transparent 40%),
    radial-gradient(circle at 70% 45%, #9682f0 0 14%, transparent 36%),
    linear-gradient(180deg, #101a2c 0 46%, #172a42 47% 72%, #101a2a 73%);
}

.bg-chip__swatch--none {
  background-image: linear-gradient(135deg, var(--panel) 0 50%, var(--panel-2) 50%);
}
</style>
