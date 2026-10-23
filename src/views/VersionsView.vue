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
  selectVersion,
  selectedVersion,
  setInstanceIcon,
  setInstanceIconImage,
  store,
  uninstallVersion,
} from "../store";
import { ICON_PRESETS, formatBytes, formatDate, iconColor, kindLabel } from "../utils";

const confirmId = ref<string | null>(null);
let confirmTimer: number | undefined;

/// 正在展开图标选择器的版本
const pickerFor = ref<string | null>(null);
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
      <div>
        <span class="eyebrow">DOCL / LIBRARY</span>
      </div>
      <div class="row">
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
        class="version-row card card--hover reveal"
        :class="{
          'version-row--active': item.id === selectedVersion?.id,
          'version-row--picking': pickerFor === item.id,
        }"
        :style="{ '--i': index }"
      >
        <span class="version-row__no mono">{{ String(index + 1).padStart(2, "0") }}</span>

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
            :size="34"
            @click="selectVersion(item.id); launchVersion(item.id)"
          />
          <IconButton
            icon="check"
            label="设为当前版本"
            :size="34"
            :disabled="item.id === selectedVersion?.id"
            @click="selectVersion(item.id)"
          />
          <IconButton
            icon="settings"
            label="版本设置"
            variant="soft"
            :size="34"
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
            variant="danger"
            :size="34"
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
  gap: 10px;
}

.version-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 18px 12px 14px;
  overflow: visible;
}

.version-row::after {
  content: "";
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--line);
  transition: background var(--dur-2) ease;
}

.version-row--active::after {
  background: var(--moss);
}

.version-row--picking {
  z-index: 6;
}

.version-row__no {
  width: 18px;
  flex-shrink: 0;
  font-size: 11px;
  letter-spacing: 0.06em;
  color: var(--muted);
  text-align: center;
  user-select: none;
}

.version-row__icon-wrap {
  position: relative;
  flex-shrink: 0;
}

.version-row__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  height: 46px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 12px;
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
  font-size: 18px;
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
  gap: 6px;
}

.picker-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
}

.icon-picker {
  position: absolute;
  left: 52px;
  top: 50px;
  z-index: 21;
  width: 268px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 12px;
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
  border-radius: 8px;
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
  border-radius: 8px;
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
  border-radius: 8px;
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
