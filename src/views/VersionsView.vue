<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openFolder } from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import InstanceGlyph from "../components/InstanceGlyph.vue";
import WoodButton from "../components/WoodButton.vue";
import {
  launchVersion,
  loadInstanceIconImage,
  openInstance,
  refreshDirCounts,
  sameDir,
  selectVersion,
  selectedVersion,
  setInstanceIcon,
  setInstanceIconImage,
  store,
  switchGameDir,
  uninstallVersion,
} from "../store";
import { ICON_PRESETS, formatBytes, formatDate, iconColor, kindLabel } from "../utils";

const confirmId = ref<string | null>(null);
let confirmTimer: number | undefined;

/// 正在展开图标选择器的版本
const pickerFor = ref<string | null>(null);
/// 目录切换面板是否展开
const dirOpen = ref(false);
/// 自定义图标的 data URL 缓存，避免每次渲染都读盘
const customIcons = reactive<Record<string, string>>({});

/// 和 PCL 一致：整合包继承的原版 / 加载器版本不单独占卡片
const visibleVersions = computed(() =>
  store.installed.filter((item) => !item.dependency)
);

function requestDelete(id: string) {
  if (confirmId.value === id) {
    confirmId.value = null;
    uninstallVersion(id);
    return;
  }
  confirmId.value = id;
  if (confirmTimer) window.clearTimeout(confirmTimer);
  confirmTimer = window.setTimeout(() => {
    if (confirmId.value === id) confirmId.value = null;
  }, 3200);
}

function openDir() {
  if (store.config?.game_dir) openFolder(store.config.game_dir);
}

function displayName(id: string) {
  const name = store.metas[id]?.display_name;
  return name && name.trim() ? name : id;
}

/// 整合包实例的 id 是包名，这里补上真实的游戏版本与加载器
function versionTag(item: { id: string; game_version?: string; loader?: string }) {
  const parts: string[] = [];
  if (item.game_version && item.game_version !== item.id) parts.push(item.game_version);
  if (item.loader && item.loader !== "vanilla") {
    parts.push(item.loader.charAt(0).toUpperCase() + item.loader.slice(1));
  }
  return parts.join(" · ");
}

const CATEGORY_LABELS: Record<string, string> = {
  survival: "生存",
  creative: "创造",
  pvp: "PvP",
  modpack: "整合包",
  tech: "生电",
  build: "建筑",
  other: "其他",
};

function categoryLabel(value?: string) {
  if (!value) return "";
  return CATEGORY_LABELS[value] ?? value;
}

function iconOf(id: string) {
  return store.metas[id]?.icon || "auto";
}

/* ---------- 游戏目录切换 ---------- */

const gameDirs = computed(() => store.config?.game_dirs ?? []);
const activeDir = computed(() => store.config?.game_dir ?? "");

async function toggleDirs() {
  if (dirOpen.value) {
    dirOpen.value = false;
    return;
  }
  pickerFor.value = null;
  dirOpen.value = true;
  await refreshDirCounts();
}

async function pickDir(path: string) {
  if (sameDir(path, activeDir.value)) {
    dirOpen.value = false;
    return;
  }
  pickerFor.value = null;
  confirmId.value = null;
  await switchGameDir(path);
  dirOpen.value = false;
  await syncIcons();
}

function manageDirs() {
  dirOpen.value = false;
  store.page = "settings";
}

function dirCount(path: string) {
  return store.dirCounts[path] ?? 0;
}

/// 元数据变化后补齐自定义图标缓存
async function syncIcons() {
  for (const item of visibleVersions.value) {
    if (iconOf(item.id) !== "custom") {
      delete customIcons[item.id];
      continue;
    }
    if (customIcons[item.id]) continue;
    const data = await loadInstanceIconImage(item.id);
    if (data) customIcons[item.id] = data;
  }
}

onMounted(syncIcons);
watch(() => store.metas, syncIcons, { deep: true });

function togglePicker(id: string) {
  pickerFor.value = pickerFor.value === id ? null : id;
}

async function choosePreset(id: string, preset: string) {
  pickerFor.value = null;
  await setInstanceIcon(id, preset);
}

async function chooseImage(id: string) {
  pickerFor.value = null;
  const source = await openDialog({
    multiple: false,
    title: "选择版本图标",
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"] }],
  });
  if (!source || typeof source !== "string") return;
  await setInstanceIconImage(id, source);
  const data = await loadInstanceIconImage(id);
  if (data) customIcons[id] = data;
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div class="page-head__actions">
        <div class="dir-switch">
          <IconButton icon="layers" label="切换游戏目录" :size="32" @click="toggleDirs" />
          <template v-if="dirOpen">
            <div class="dir-backdrop" @click="dirOpen = false" />
            <div class="dir-panel">
              <span class="dir-panel__title">游戏目录</span>
              <button
                v-for="dir in gameDirs"
                :key="dir.path"
                class="dir-item"
                :class="{ 'dir-item--active': sameDir(dir.path, activeDir) }"
                @click="pickDir(dir.path)"
              >
                <span class="dir-item__body">
                  <strong>{{ dir.name }}</strong>
                  <span class="dir-item__path mono">{{ dir.path }}</span>
                </span>
                <span class="dir-item__count mono">{{ dirCount(dir.path) }}</span>
                <AppIcon
                  v-if="sameDir(dir.path, activeDir)"
                  name="check"
                  :size="14"
                />
              </button>
              <button class="dir-manage" @click="manageDirs">管理目录…</button>
            </div>
          </template>
        </div>
        <IconButton icon="folder" label="打开游戏目录" :size="32" @click="openDir" />
        <WoodButton variant="primary" size="sm" sheen @click="store.page = 'download'">
          <AppIcon name="plus" :size="14" />
          安装新版本
        </WoodButton>
      </div>
    </header>

    <div v-if="!visibleVersions.length" class="blank">
      <p class="blank__title">版本架还是空的</p>
      <p class="blank__hint">从右上角安装新版本，或直接把整合包压缩包拖进窗口</p>
    </div>

    <div v-else class="version-list">
      <article
        v-for="(item, index) in visibleVersions"
        :key="item.id"
        class="version-row reveal"
        :class="{
          'version-row--active': item.id === selectedVersion?.id,
          'version-row--picking': pickerFor === item.id,
        }"
        :style="{ '--i': index }"
      >
        <div class="version-row__icon-wrap">
          <button
            class="version-row__icon"
            :style="{ '--tile': iconColor(iconOf(item.id)) }"
            :title="`更换 ${displayName(item.id)} 的图标`"
            @click.stop="togglePicker(item.id)"
          >
            <img v-if="customIcons[item.id]" :src="customIcons[item.id]" alt="" />
            <InstanceGlyph v-else :preset="iconOf(item.id)" :size="26" />
          </button>

          <template v-if="pickerFor === item.id">
            <div class="picker-backdrop" @click.stop="pickerFor = null" />
            <div class="icon-picker" @click.stop>
              <span class="icon-picker__title">选择图标</span>
              <div class="icon-picker__grid">
                <button
                  v-for="preset in ICON_PRESETS"
                  :key="preset.value"
                  class="icon-picker__chip"
                  :class="{ 'icon-picker__chip--active': iconOf(item.id) === preset.value }"
                  @click="choosePreset(item.id, preset.value)"
                >
                  <span class="icon-picker__tile" :style="{ '--tile': preset.color }">
                    <InstanceGlyph :preset="preset.value" :size="18" />
                  </span>
                  {{ preset.label }}
                </button>
              </div>
              <button class="icon-picker__upload" @click="chooseImage(item.id)">
                <AppIcon name="image" :size="14" />
                自定义图片…
              </button>
            </div>
          </template>
        </div>

        <div class="version-row__main">
          <div class="version-row__title">
            <strong class="display">{{ displayName(item.id) }}</strong>
            <span v-if="store.metas[item.id]?.favorite" class="tag tag--grass">收藏</span>
            <span v-if="item.id === selectedVersion?.id" class="tag tag--grass">当前</span>
            <span class="tag">{{ kindLabel(item.kind) }}</span>
          </div>
          <span class="version-row__sub mono">
            <template v-if="versionTag(item)">{{ versionTag(item) }} · </template>
            {{ item.id }}
            <template v-if="store.metas[item.id]?.category && store.metas[item.id]?.category !== 'auto'">
              · {{ categoryLabel(store.metas[item.id]?.category) }}
            </template>
          </span>
        </div>

        <div class="version-row__meta">
          <span><i>客户端</i>{{ formatBytes(item.jar_size) }}</span>
          <span><i>Java</i>{{ item.java_major }}</span>
          <span><i>安装于</i>{{ formatDate(new Date(item.last_modified * 1000).toISOString()) }}</span>
        </div>

        <div class="version-row__actions">
          <IconButton
            icon="play"
            :label="`启动 ${item.id}`"
            variant="primary"
            :size="30"
            @click="selectVersion(item.id); launchVersion(item.id)"
          />
          <IconButton
            icon="check"
            label="设为当前版本"
            :size="30"
            :disabled="item.id === selectedVersion?.id"
            @click="selectVersion(item.id)"
          />
          <IconButton
            icon="settings"
            label="版本设置"
            variant="soft"
            :size="30"
            @click="openInstance(item.id)"
          />
          <IconButton
            icon="trash"
            :label="
              confirmId === item.id
                ? '确认删除'
                : store.metas[item.id]?.category === 'modpack'
                  ? '删除整合包'
                  : '删除版本'
            "
            variant="danger-quiet"
            :size="30"
            @click="requestDelete(item.id)"
          />
        </div>
      </article>
    </div>
  </div>
</template>

<style scoped>
.blank {
  margin-top: 18vh;
  text-align: center;
  user-select: none;
}

.dir-switch {
  position: relative;
  display: flex;
}

.dir-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
}

.dir-panel {
  position: absolute;
  right: 0;
  top: calc(100% + 10px);
  z-index: 21;
  width: 320px;
  padding: 8px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  box-shadow: 0 14px 34px rgba(52, 38, 24, 0.22);
}

.dir-panel__title {
  display: block;
  padding: 6px 8px 8px;
  font-family: var(--font-mono);
  font-size: 9.5px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--muted);
}

.dir-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 10px;
  border: 1px solid transparent;
  border-radius: 4px;
  background: transparent;
  text-align: left;
  cursor: pointer;
  transition:
    background var(--dur-2) ease,
    border-color var(--dur-2) ease;
}

.dir-item:hover {
  background: rgba(var(--accent-rgb), 0.06);
}

.dir-item--active {
  border-color: var(--moss);
  background: rgba(var(--accent-rgb), 0.08);
}

.dir-item__body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.dir-item__body strong {
  font-size: 13px;
  color: var(--walnut-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-item__path {
  font-size: 9.5px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-item__count {
  flex-shrink: 0;
  font-size: 10.5px;
  color: var(--muted);
}

.dir-manage {
  display: block;
  width: 100%;
  margin-top: 4px;
  padding: 8px 0;
  border: none;
  border-top: 1px dashed var(--line);
  border-radius: 0 0 8px 8px;
  background: transparent;
  font-size: 11.5px;
  color: var(--ink-soft);
  cursor: pointer;
  transition: color var(--dur-2) ease;
}

.dir-manage:hover {
  color: var(--moss);
}

.blank__title {
  font-size: 19px;
  letter-spacing: 0.18em;
  color: var(--ink-soft);
}

.blank__hint {
  margin-top: 10px;
  font-size: 12.5px;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.version-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  /* 整块浅色面板兜底，毛玻璃/无底图下行内容也能看清 */
  background: rgba(var(--glass-rgb), 0.88);
}

/* PCL 式紧凑行：默认无框透明，悬停/选中才铺浅色 */
.version-row {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px 8px 16px;
  overflow: visible;
  border: 1px solid transparent;
  border-radius: 4px;
  background: transparent;
  transition: background var(--dur-2) ease;
}

.version-row:hover {
  background: rgba(var(--accent-rgb), 0.05);
}

.version-row--active {
  background: rgba(var(--accent-rgb), 0.08);
}

.version-row::after {
  content: "";
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: transparent;
  transition: background var(--dur-2) ease;
}

.version-row--active::after {
  background: var(--moss);
}

.version-row--picking {
  z-index: 6;
}

.version-row__icon-wrap {
  position: relative;
  flex-shrink: 0;
}

.version-row__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: color-mix(in srgb, var(--tile) 16%, rgba(255, 255, 255, 0.55));
  cursor: pointer;
  overflow: hidden;
  transition:
    transform var(--dur-2) ease,
    border-color var(--dur-2) ease,
    box-shadow var(--dur-2) ease;
}

.version-row__icon:hover {
  transform: translateY(-1px);
  border-color: var(--tile);
  box-shadow: 0 4px 12px rgba(var(--accent-rgb), 0.16);
}

.version-row__icon img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.version-row__main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1 1 240px;
}

.version-row__title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.version-row__title strong {
  font-size: 15.5px;
  line-height: 1.25;
  color: var(--walnut-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.version-row__sub {
  font-size: 10.5px;
  color: var(--muted);
  letter-spacing: 0.04em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.version-row__meta {
  display: flex;
  flex: 0 0 auto;
  gap: 16px;
  color: var(--ink-soft);
  font-size: 12px;
  white-space: nowrap;
}

.version-row__meta i {
  font-style: normal;
  font-family: var(--font-mono);
  font-size: 9px;
  letter-spacing: 0.1em;
  color: var(--muted);
  margin-right: 5px;
  text-transform: uppercase;
}

.version-row__actions {
  display: flex;
  flex-shrink: 0;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--dur-2) ease;
}

.version-row:hover .version-row__actions,
.version-row:focus-within .version-row__actions,
.version-row--picking .version-row__actions {
  opacity: 1;
}

/* PCL 式浅显图标：去边框去底色，只留线条图案 */
.version-row__actions :deep(.icon-btn--primary),
.version-row__actions :deep(.icon-btn--soft),
.version-row__actions :deep(.icon-btn--danger) {
  border-color: transparent;
  background: transparent;
  box-shadow: none;
}

.version-row__actions :deep(.icon-btn--primary) {
  color: var(--moss);
}

.version-row__actions :deep(.icon-btn--danger) {
  color: var(--danger);
}

.version-row__actions :deep(.icon-btn--primary:hover:not(:disabled)),
.version-row__actions :deep(.icon-btn--soft:hover:not(:disabled)) {
  border-color: transparent;
  background: rgba(var(--accent-rgb), 0.08);
  box-shadow: none;
  color: var(--walnut-deep);
}

.version-row__actions :deep(.icon-btn--danger:hover:not(:disabled)) {
  border-color: transparent;
  background: rgba(190, 92, 70, 0.1);
  box-shadow: none;
  color: var(--danger);
}

.picker-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
}

.icon-picker {
  position: absolute;
  left: 46px;
  top: 46px;
  z-index: 21;
  width: 268px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel);
  box-shadow: 0 14px 34px rgba(52, 38, 24, 0.22);
}

.icon-picker__title {
  display: block;
  margin-bottom: 8px;
  font-family: var(--font-mono);
  font-size: 9.5px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--muted);
}

.icon-picker__grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
}

.icon-picker__chip {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 6px 2px;
  border: 1px solid transparent;
  border-radius: 4px;
  background: transparent;
  font-size: 10.5px;
  color: var(--ink-soft);
  cursor: pointer;
  transition:
    background var(--dur-2) ease,
    border-color var(--dur-2) ease;
}

.icon-picker__chip:hover {
  background: rgba(var(--accent-rgb), 0.06);
}

.icon-picker__chip--active {
  border-color: var(--moss);
  background: rgba(var(--accent-rgb), 0.08);
}

.icon-picker__tile {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--tile) 18%, rgba(255, 255, 255, 0.6));
}

.icon-picker__upload {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 100%;
  margin-top: 10px;
  padding: 7px 0;
  border: 1px dashed var(--line);
  border-radius: 4px;
  background: transparent;
  font-size: 11.5px;
  color: var(--ink-soft);
  cursor: pointer;
  transition:
    color var(--dur-2) ease,
    border-color var(--dur-2) ease;
}

.icon-picker__upload:hover {
  color: var(--moss);
  border-color: var(--moss);
}

@media (max-width: 1180px) {
  .version-row__meta {
    display: none;
  }
}
</style>
