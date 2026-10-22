<script setup lang="ts">
import { nextTick, reactive, ref, watch } from "vue";
import { api, pickDirectory, pickImage, type Config } from "../api";
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
  refreshJava,
  saveConfig,
  store,
} from "../store";

const form = reactive<Config>({
  game_dir: "",
  java_path: null,
  memory_mb: 4096,
  java_args: "",
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
});

watch(
  () => store.config,
  (config) => {
    if (config) Object.assign(form, config);
  },
  { immediate: true }
);

const runtimeMajor = ref("21");
const saving = ref(false);
const runtimeBusy = ref(false);

type SettingsTab = "game" | "java" | "appearance" | "performance" | "download";
const settingsTab = ref<SettingsTab>("game");

const settingsTabs: { value: SettingsTab; label: string; icon: string; no: string }[] = [
  { value: "game", label: "游戏与启动", icon: "cube", no: "01" },
  { value: "java", label: "Java 运行时", icon: "database", no: "02" },
  { value: "appearance", label: "外观", icon: "image", no: "03" },
  { value: "performance", label: "性能与参数", icon: "sun", no: "04" },
  { value: "download", label: "下载", icon: "download", no: "05" },
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
  { value: "nocturne", label: "夜墨", colors: ["#1d1a16", "#c08a55", "#88a867"] },
];

const builtinBackgrounds = [
  { value: "forest", label: "林间晨雾" },
  { value: "dune", label: "赤陶沙丘" },
  { value: "night", label: "夜航星空" },
];

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

async function chooseBackground() {
  const path = await pickImage();
  if (!path) return;
  try {
    const info = await api.importBackground(path);
    store.backgroundPreset = "";
    store.backgroundUrl = info.data_url;
    form.background_image = info.path;
    notify("背景已应用，保存后长期保留");
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
  notify("已移除背景，保存后生效");
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

async function chooseDir() {
  const dir = await pickDirectory();
  if (dir) form.game_dir = dir;
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

async function save() {
  if (!form.game_dir.trim()) {
    notify("请先选择游戏目录", "err");
    return;
  }
  saving.value = true;
  try {
    await saveConfig({ ...form });
    await loadBackground();
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
        <h1 class="page-title">设置</h1>
        <p class="page-sub">启动参数、Java 运行时与下载策略</p>
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

      <div class="setting">
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

      <div class="setting">
        <div class="setting__label">
          <strong>主题配色</strong>
          <span>即时预览，保存后长期生效</span>
        </div>
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

      <div class="setting">
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

      <div class="setting">
        <div class="setting__label">
          <strong>最大内存</strong>
          <span>{{ form.memory_mb }} MB（约 {{ (form.memory_mb / 1024).toFixed(1) }} GB）</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodSlider v-model="form.memory_mb" :min="1024" :max="16384" :step="512" />
        </div>
      </div>
      <div class="setting setting--last">
        <div class="setting__label">
          <strong>额外 JVM 参数</strong>
          <span>空格分隔，例如 -XX:+UseZGC</span>
        </div>
        <div class="setting__control setting__control--wide">
          <WoodInput v-model="form.java_args" mono placeholder="-XX:+UseG1GC" />
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
      </div>
    </div>
  </div>
</template>

<style scoped>
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

.resolution__x {
  font-size: 13px;
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
  justify-content: flex-end;
  max-width: 470px;
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
  justify-content: flex-end;
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

.bg-chip__swatch--none {
  background-image: linear-gradient(135deg, var(--panel) 0 50%, var(--panel-2) 50%);
}
</style>
